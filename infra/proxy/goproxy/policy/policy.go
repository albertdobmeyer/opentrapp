// Package policy is the pure, network-free L7 egress policy for vault-proxy —
// originally a faithful port of the mitmproxy addon vault-proxy.py (since removed;
// the goproxy is now the sole L7 implementation, ADR-0026). The "Mirrors
// vault-proxy.py …" notes below name the original behaviour contract this port
// preserves. Keeping the decisions pure (no I/O, no goproxy types) makes them
// unit-pinnable (policy_test.go), the same contract as the #182 pins.
package policy

import (
	"bytes"
	"net"
	"regexp"
	"strings"
)

const (
	// Mirrors vault-proxy.py EXFIL_THRESHOLD_BYTES / EXFIL_RESPONSE_THRESHOLD_BYTES.
	ExfilRequestThreshold  = 1 * 1024 * 1024  // 1 MB — block large outbound payloads
	ExfilResponseThreshold = 10 * 1024 * 1024 // 10 MB — block large responses
	RedactedMarker         = "[REDACTED_BY_VAULT]"
)

// InjectionRule injects provider auth headers into requests to matching hosts,
// pulling the secret from an env var set ONLY in the proxy container.
type InjectionRule struct {
	Match   []string          // host suffixes — matched exactly or as ".suffix"
	EnvVar  string            // env var holding the secret ("" => no injection)
	Headers map[string]string // header name -> template; literal "{key}" -> secret
}

// botTokenRe redacts a Telegram bot token embedded in the URL path
// (https://api.telegram.org/bot<id>:<hash>/...). Mirrors vault-proxy.py.
var botTokenRe = regexp.MustCompile(`(/bot)\d+:[A-Za-z0-9_-]{20,}`)

// privateNets — the DNS-rebinding destination filter (ADR-0009 Tier 2). Exact
// port of vault-proxy.py _PRIVATE_DEST_NETWORKS.
var privateNets []*net.IPNet

func init() {
	for _, cidr := range []string{
		"0.0.0.0/8",     // "This network"
		"10.0.0.0/8",    // RFC1918
		"100.64.0.0/10", // Carrier-grade NAT (RFC6598)
		"127.0.0.0/8",   // IPv4 loopback
		"169.254.0.0/16", // Link-local + AWS/GCP metadata (169.254.169.254)
		"172.16.0.0/12", // RFC1918 — also the default docker/podman bridge
		"192.0.0.0/24",  // IETF protocol assignments
		"192.168.0.0/16", // RFC1918
		"198.18.0.0/15", // Benchmark testing
		"224.0.0.0/4",   // IPv4 multicast
		"240.0.0.0/4",   // Reserved
		"::1/128",       // IPv6 loopback
		"fc00::/7",      // IPv6 unique local
		"fe80::/10",     // IPv6 link-local
		"ff00::/8",      // IPv6 multicast
	} {
		_, n, err := net.ParseCIDR(cidr)
		if err != nil {
			panic("policy: bad CIDR " + cidr + ": " + err.Error())
		}
		privateNets = append(privateNets, n)
	}
}

// IsAllowed reports whether host matches an allowlisted domain (exact or
// subdomain). Raw IP literals are always rejected (the allowlist is domain-only).
// Mirrors vault-proxy.py _is_allowed.
func IsAllowed(host string, allowlist map[string]bool) bool {
	host = strings.ToLower(host)
	if net.ParseIP(strings.Trim(host, "[]")) != nil {
		return false // raw IP — allowlist is domain-only
	}
	for allowed := range allowlist {
		if host == allowed || strings.HasSuffix(host, "."+allowed) {
			return true
		}
	}
	return false
}

// IsPrivateIP reports whether ip falls in any private/loopback/link-local/
// multicast range. Mirrors the membership test in vault-proxy.py _resolves_to_private.
func IsPrivateIP(ip net.IP) bool {
	for _, n := range privateNets {
		if n.Contains(ip) {
			return true
		}
	}
	return false
}

// MatchInjection returns the first rule whose suffixes match host (exact or
// subdomain), or nil. Mirrors the request()-loop in vault-proxy.py (first match wins).
func MatchInjection(host string, rules []InjectionRule) *InjectionRule {
	host = strings.ToLower(host)
	for i := range rules {
		for _, suffix := range rules[i].Match {
			s := strings.ToLower(suffix)
			if host == s || strings.HasSuffix(host, "."+s) {
				return &rules[i]
			}
		}
	}
	return nil
}

// RedactURL redacts a Telegram bot token embedded in the URL path.
func RedactURL(url string) string {
	return botTokenRe.ReplaceAllString(url, "${1}<REDACTED_BOT_TOKEN>")
}

// RedactKeys replaces every occurrence of each non-empty secret in data with the
// redaction marker, returning the (possibly) rewritten bytes and whether anything
// was redacted. Mirrors vault-proxy.py response()-body key redaction.
func RedactKeys(data []byte, secrets []string) ([]byte, bool) {
	redacted := false
	marker := []byte(RedactedMarker)
	for _, s := range secrets {
		if s == "" {
			continue
		}
		sb := []byte(s)
		if bytes.Contains(data, sb) {
			data = bytes.ReplaceAll(data, sb, marker)
			redacted = true
		}
	}
	return data, redacted
}

// RequestDecision is the outcome of the request-path policy.
type RequestDecision struct {
	Block  bool
	Status int            // 403 (allowlist/rebinding) or 413 (exfil), when Block
	Reason string         // human-readable reason, when Block
	Inject *InjectionRule // the matched injection rule, when allowed (may be nil)
}

// DecideRequest applies the request-path policy in the SAME ORDER as
// vault-proxy.py request(): allowlist -> DNS-rebinding -> outbound size limit
// -> injection. The size check is BEFORE injection on purpose, so a blocked
// oversized request never has a real key attached (the #182 pin). The DNS result
// is passed in (resolvesToPrivate) so this stays pure and testable.
func DecideRequest(host string, allowlist map[string]bool, resolvesToPrivate bool, requestSize int, rules []InjectionRule) RequestDecision {
	if !IsAllowed(host, allowlist) {
		return RequestDecision{Block: true, Status: 403, Reason: "domain not in allowlist"}
	}
	if resolvesToPrivate {
		return RequestDecision{Block: true, Status: 403, Reason: "resolves to a private address (DNS rebinding)"}
	}
	if requestSize > ExfilRequestThreshold {
		return RequestDecision{Block: true, Status: 413, Reason: "outbound payload exceeds exfiltration threshold"}
	}
	return RequestDecision{Block: false, Inject: MatchInjection(host, rules)}
}
