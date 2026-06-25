// Command vault-proxy is the L7 egress chokepoint of the OpenTrApp perimeter:
// a goproxy-based MITM forward proxy that injects API keys into the contained
// agent's HTTPS (the credential never enters the agent container), enforces a
// domain allowlist + a DNS-rebinding filter + outbound/response size caps +
// reflected-key redaction, logs every request as JSON, and chains UPSTREAM to
// vault-egress (no direct internet). It replaces the leaky Python mitmproxy
// (ADR-0026). The pure policy is in ./policy (pinned); this file wires it.
package main

import (
	"bytes"
	"crypto/tls"
	"crypto/x509"
	"encoding/json"
	"fmt"
	"io"
	"log"
	"net"
	"net/http"
	"net/url"
	"os"
	"os/signal"
	"path/filepath"
	"strconv"
	"strings"
	"sync/atomic"
	"syscall"
	"time"

	"github.com/elazarl/goproxy"

	"vault-proxy/policy"
)

const listenAddr = "0.0.0.0:8080"

// Config locates the proxy's mounted files + upstream. Defaults are the perimeter
// paths; tests override them with temp dirs + a test upstream.
type Config struct {
	AllowlistPath  string
	InjectionPath  string
	CADir          string // the proxy-ca volume
	LogDir         string
	Upstream       string
	UpstreamRootCAs *x509.CertPool // nil => system roots (production); tests set the origin pool
}

func defaultConfig() Config {
	return Config{
		AllowlistPath: "/opt/vault/allowlist.txt",
		InjectionPath: "/opt/vault/injection.json",
		CADir:         "/home/mitmproxy/.mitmproxy",
		LogDir:        "/var/log/vault-proxy",
		Upstream:      getenv("VAULT_UPSTREAM", "http://vault-egress:8888"),
	}
}

var (
	allowlist  atomic.Pointer[map[string]bool]
	rules      []policy.InjectionRule
	secrets    []string // injected key VALUES, for response redaction
	jsonLogger *log.Logger
	activeCfg  Config // for SIGHUP reload
)

// buildProxy wires the goproxy server from cfg: CA, allowlist, rules, handlers,
// upstream chaining. Returns a proxy ready to serve.
func buildProxy(cfg Config) (*goproxy.ProxyHttpServer, error) {
	activeCfg = cfg
	setupLogger(cfg.LogDir)
	loadAllowlist(cfg.AllowlistPath)
	loadRules(cfg.InjectionPath)

	ca, err := loadOrCreateCA(cfg.CADir)
	if err != nil {
		return nil, fmt.Errorf("CA: %w", err)
	}
	setGoproxyCA(ca)

	upURL, err := url.Parse(cfg.Upstream)
	if err != nil {
		return nil, fmt.Errorf("bad upstream %q: %w", cfg.Upstream, err)
	}
	proxy := goproxy.NewProxyHttpServer()
	proxy.OnRequest().HandleConnect(goproxy.AlwaysMitm)
	proxy.Tr = &http.Transport{
		Proxy:           http.ProxyURL(upURL), // chain to vault-egress
		TLSClientConfig: &tls.Config{MinVersion: tls.VersionTLS12, RootCAs: cfg.UpstreamRootCAs},
	}
	proxy.OnRequest().DoFunc(onRequest)
	proxy.OnResponse().DoFunc(onResponse)
	return proxy, nil
}

func main() {
	cfg := defaultConfig()
	proxy, err := buildProxy(cfg)
	if err != nil {
		log.Fatalf("[vault-proxy] %v", err)
	}

	// SIGHUP -> atomic allowlist reload (matches the mitmproxy addon).
	hup := make(chan os.Signal, 1)
	signal.Notify(hup, syscall.SIGHUP)
	go func() {
		for range hup {
			loadAllowlist(activeCfg.AllowlistPath)
			logEvent(map[string]any{"action": "ALLOWLIST_RELOADED", "count": len(*allowlist.Load())})
		}
	}()

	log.Printf("[vault-proxy] listening on %s -> upstream %s (%d allow, %d rule(s))",
		listenAddr, cfg.Upstream, len(*allowlist.Load()), len(rules))
	if err := http.ListenAndServe(listenAddr, proxy); err != nil {
		log.Fatalf("[vault-proxy] serve: %v", err)
	}
}

// onRequest applies the request-path policy in the pinned order: allowlist ->
// DNS-rebinding -> outbound size cap (BEFORE injection) -> inject.
func onRequest(req *http.Request, _ *goproxy.ProxyCtx) (*http.Request, *http.Response) {
	host := hostname(req)
	al := *allowlist.Load()

	// Buffer the body (capped) so we can size-check chunked/unknown-length bodies
	// without unbounded memory, and reset it for forwarding.
	var size int
	if req.Body != nil {
		buf, _ := io.ReadAll(io.LimitReader(req.Body, policy.ExfilRequestThreshold+1))
		req.Body.Close()
		req.Body = io.NopCloser(bytes.NewReader(buf))
		req.ContentLength = int64(len(buf))
		size = len(buf)
	}

	priv := false
	if policy.IsAllowed(host, al) {
		priv = resolvesToPrivate(host)
	}
	dec := policy.DecideRequest(host, al, priv, size, rules)
	redactedURL := policy.RedactURL(req.URL.String())
	if dec.Block {
		logEvent(map[string]any{
			"action": blockAction(dec.Status), "method": req.Method,
			"url": redactedURL, "host": host, "request_bytes": size, "reason": dec.Reason,
		})
		return req, blockResponse(req, dec.Status, dec.Reason)
	}
	if dec.Inject != nil {
		injectHeaders(req, dec.Inject)
	}
	logEvent(map[string]any{
		"action": "ALLOWED", "method": req.Method,
		"url": redactedURL, "host": host, "request_bytes": size,
	})
	return req, nil
}

// onResponse caps response size and redacts any reflected API key (headers + body).
func onResponse(resp *http.Response, ctx *goproxy.ProxyCtx) *http.Response {
	if resp == nil {
		return resp
	}
	redactedURL := ""
	if resp.Request != nil {
		redactedURL = policy.RedactURL(resp.Request.URL.String())
	}

	body, _ := io.ReadAll(io.LimitReader(resp.Body, policy.ExfilResponseThreshold+1))
	resp.Body.Close()
	if len(body) > policy.ExfilResponseThreshold {
		logEvent(map[string]any{"action": "LARGE_RESPONSE_BLOCKED", "url": redactedURL,
			"response_bytes": len(body), "reason": "response exceeds threshold"})
		return goproxy.NewResponse(resp.Request, "text/plain", http.StatusRequestEntityTooLarge, "Response too large")
	}

	// Redact reflected keys in headers, then body.
	keyRedacted := false
	for name, vals := range resp.Header {
		for i, v := range vals {
			if nv, did := policy.RedactKeys([]byte(v), secrets); did {
				resp.Header[name][i] = string(nv)
				keyRedacted = true
			}
		}
	}
	newBody, didBody := policy.RedactKeys(body, secrets)
	if didBody || keyRedacted {
		logEvent(map[string]any{"action": "KEY_REFLECTED", "url": redactedURL,
			"reason": "API key found in response — redacted"})
	}
	resp.Body = io.NopCloser(bytes.NewReader(newBody))
	resp.ContentLength = int64(len(newBody))
	resp.Header.Set("Content-Length", strconv.Itoa(len(newBody)))

	logEvent(map[string]any{"action": "RESPONSE", "url": redactedURL,
		"status": resp.StatusCode, "response_bytes": len(newBody)})
	return resp
}

// lookupIP is the resolver used by the DNS-rebinding defense; overridable in tests.
var lookupIP = net.LookupIP

// resolvesToPrivate: DNS-rebinding defense. Fail-closed on any resolution error.
func resolvesToPrivate(host string) bool {
	host = strings.Trim(host, "[]")
	if ip := net.ParseIP(host); ip != nil {
		return policy.IsPrivateIP(ip)
	}
	ips, err := lookupIP(host)
	if err != nil || len(ips) == 0 {
		return true // fail-closed
	}
	for _, ip := range ips {
		if policy.IsPrivateIP(ip) {
			return true
		}
	}
	return false
}

func injectHeaders(req *http.Request, rule *policy.InjectionRule) {
	key := os.Getenv(rule.EnvVar)
	if key == "" {
		return // missing key -> request goes out unauthenticated; a missing key never leaks
	}
	for h, tmpl := range rule.Headers {
		req.Header.Set(h, strings.ReplaceAll(tmpl, "{key}", key))
	}
}

func setGoproxyCA(ca tls.Certificate) {
	goproxy.GoproxyCa = ca
	tlsConfig := goproxy.TLSConfigFromCA(&ca)
	goproxy.OkConnect = &goproxy.ConnectAction{Action: goproxy.ConnectAccept, TLSConfig: tlsConfig}
	goproxy.MitmConnect = &goproxy.ConnectAction{Action: goproxy.ConnectMitm, TLSConfig: tlsConfig}
	goproxy.HTTPMitmConnect = &goproxy.ConnectAction{Action: goproxy.ConnectHTTPMitm, TLSConfig: tlsConfig}
	goproxy.RejectConnect = &goproxy.ConnectAction{Action: goproxy.ConnectReject, TLSConfig: tlsConfig}
}

func loadAllowlist(path string) {
	m := map[string]bool{}
	if data, err := os.ReadFile(path); err == nil {
		for _, line := range strings.Split(string(data), "\n") {
			d := strings.TrimSpace(line)
			if d != "" && !strings.HasPrefix(d, "#") {
				m[strings.ToLower(d)] = true
			}
		}
	} else {
		log.Printf("[vault-proxy] allowlist %s unreadable (%v) — blocking ALL", path, err)
	}
	allowlist.Store(&m)
}

func loadRules(injPath string) {
	rules = []policy.InjectionRule{
		{Match: []string{"api.anthropic.com"}, EnvVar: "ANTHROPIC_API_KEY",
			Headers: map[string]string{"x-api-key": "{key}", "anthropic-version": getenv("ANTHROPIC_API_VERSION", "2023-06-01")}},
		{Match: []string{"api.openai.com"}, EnvVar: "OPENAI_API_KEY",
			Headers: map[string]string{"Authorization": "Bearer {key}"}},
	}
	// Optional mounted extension (BYOK providers) — fail-safe: malformed/absent => defaults stand.
	if data, err := os.ReadFile(injPath); err == nil {
		var extra []policy.InjectionRule
		if json.Unmarshal(data, &extra) == nil {
			for _, r := range extra {
				if len(r.Match) > 0 && r.EnvVar != "" && len(r.Headers) > 0 {
					rules = append(rules, r)
				}
			}
		} else {
			log.Printf("[vault-proxy] %s present but malformed — using in-image defaults only", injPath)
		}
	}
	// The set of key VALUES to redact from responses.
	seen := map[string]bool{}
	for _, r := range rules {
		if v := os.Getenv(r.EnvVar); v != "" && !seen[v] {
			secrets = append(secrets, v)
			seen[v] = true
		}
	}
}

func setupLogger(logDir string) {
	path := filepath.Join(logDir, "requests.jsonl")
	if err := os.MkdirAll(logDir, 0o755); err == nil {
		if f, err := os.OpenFile(path, os.O_CREATE|os.O_APPEND|os.O_WRONLY, 0o644); err == nil {
			jsonLogger = log.New(f, "", 0)
			return
		}
	}
	// Loud fallback — a silent /tmp fallback is what hid the ZONE-3 bug for weeks
	// (host-side idle auto-pause reads the volume file).
	fb := "/tmp/vault-proxy-requests.jsonl"
	log.Printf("[vault-proxy] ZONE-3 ALERT: %s not writable; falling back to %s (EPHEMERAL). "+
		"Host-side idle auto-pause is DISABLED while this persists — check the log-dir chown.", path, fb)
	f, _ := os.OpenFile(fb, os.O_CREATE|os.O_APPEND|os.O_WRONLY, 0o644)
	jsonLogger = log.New(f, "", 0)
}

func logEvent(ev map[string]any) {
	ev["timestamp"] = time.Now().Format("2006-01-02T15:04:05-0700")
	ev["ts_ms"] = time.Now().UnixMilli() // epoch ms — the idle-signal reader parses this
	if b, err := json.Marshal(ev); err == nil {
		jsonLogger.Println(string(b))
		log.Printf("[vault-proxy] %s", b)
	}
}

func blockResponse(req *http.Request, status int, reason string) *http.Response {
	body, _ := json.Marshal(map[string]any{"error": "blocked", "reason": reason})
	return goproxy.NewResponse(req, "application/json", status, string(body))
}

func blockAction(status int) string {
	if status == http.StatusRequestEntityTooLarge {
		return "EXFIL_BLOCKED"
	}
	return "BLOCKED"
}

func hostname(req *http.Request) string {
	h := req.URL.Hostname()
	if h == "" {
		h = req.Host
		if i := strings.LastIndex(h, ":"); i != -1 && !strings.Contains(h, "]") {
			h = h[:i]
		}
	}
	return h
}

func getenv(k, def string) string {
	if v := os.Getenv(k); v != "" {
		return v
	}
	return def
}
