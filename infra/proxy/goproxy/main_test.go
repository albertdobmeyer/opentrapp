package main

import (
	"bytes"
	"io"
	"net"
	"net/http"
	"os"
	"path/filepath"
	"strings"
	"testing"

	"vault-proxy/policy"
)

// testSetup wires the package globals for handler tests without the perimeter paths.
func testSetup(t *testing.T) {
	t.Helper()
	setupLogger(t.TempDir())
	al := map[string]bool{"api.anthropic.com": true, "raw.githubusercontent.com": true}
	allowlist.Store(&al)
	rules = []policy.InjectionRule{
		{Match: []string{"api.anthropic.com"}, EnvVar: "ANTHROPIC_API_KEY",
			Headers: map[string]string{"x-api-key": "{key}", "anthropic-version": "2023-06-01"}},
	}
	// Resolver returns a public IP so allowlisted hosts pass the rebinding check.
	lookupIP = func(string) ([]net.IP, error) { return []net.IP{net.ParseIP("8.8.8.8")}, nil }
	t.Cleanup(func() { lookupIP = net.LookupIP })
}

func mkReq(t *testing.T, host string, bodyLen int) *http.Request {
	t.Helper()
	req, err := http.NewRequest("POST", "https://"+host+"/v1/messages", strings.NewReader(strings.Repeat("x", bodyLen)))
	if err != nil {
		t.Fatal(err)
	}
	return req
}

func TestOnRequest_offAllowlistBlocked(t *testing.T) {
	testSetup(t)
	_, resp := onRequest(mkReq(t, "evil.example.com", 10), nil)
	if resp == nil || resp.StatusCode != http.StatusForbidden {
		t.Fatalf("off-allowlist: want 403, got %v", resp)
	}
}

func TestOnRequest_injectsKey(t *testing.T) {
	testSetup(t)
	os.Setenv("ANTHROPIC_API_KEY", "sk-secret-KEY")
	defer os.Unsetenv("ANTHROPIC_API_KEY")
	got, resp := onRequest(mkReq(t, "api.anthropic.com", 10), nil)
	if resp != nil {
		t.Fatalf("allowlisted request must not be blocked: %v", resp)
	}
	if got.Header.Get("x-api-key") != "sk-secret-KEY" {
		t.Errorf("x-api-key not injected: %q", got.Header.Get("x-api-key"))
	}
	if got.Header.Get("anthropic-version") != "2023-06-01" {
		t.Errorf("anthropic-version not set: %q", got.Header.Get("anthropic-version"))
	}
}

func TestOnRequest_oversizedBlockedBeforeInjection(t *testing.T) {
	testSetup(t)
	os.Setenv("ANTHROPIC_API_KEY", "sk-secret-KEY")
	defer os.Unsetenv("ANTHROPIC_API_KEY")
	got, resp := onRequest(mkReq(t, "api.anthropic.com", policy.ExfilRequestThreshold+1), nil)
	if resp == nil || resp.StatusCode != http.StatusRequestEntityTooLarge {
		t.Fatalf("oversized: want 413, got %v", resp)
	}
	if got.Header.Get("x-api-key") != "" {
		t.Error("oversized blocked request must NOT carry the injected key (key never rides a blocked exfil)")
	}
}

func TestOnResponse_redactsReflectedKey(t *testing.T) {
	testSetup(t)
	secrets = []string{"sk-secret-KEY"}
	resp := &http.Response{
		StatusCode: 200,
		Header:     http.Header{"Content-Type": []string{"application/json"}},
		Body:       io.NopCloser(strings.NewReader(`{"echo":"sk-secret-KEY leaked"}`)),
		Request:    mkReq(t, "api.anthropic.com", 0),
	}
	out := onResponse(resp, nil)
	body, _ := io.ReadAll(out.Body)
	if strings.Contains(string(body), "sk-secret-KEY") {
		t.Errorf("key not redacted from response body: %q", body)
	}
	if !strings.Contains(string(body), policy.RedactedMarker) {
		t.Errorf("redaction marker missing: %q", body)
	}
}

func TestOnResponse_oversizedBlocked(t *testing.T) {
	testSetup(t)
	secrets = nil
	resp := &http.Response{
		StatusCode: 200,
		Header:     http.Header{},
		Body:       io.NopCloser(strings.NewReader(strings.Repeat("y", policy.ExfilResponseThreshold+1))),
		Request:    mkReq(t, "api.anthropic.com", 0),
	}
	out := onResponse(resp, nil)
	if out.StatusCode != http.StatusRequestEntityTooLarge {
		t.Fatalf("oversized response: want 413, got %d", out.StatusCode)
	}
}

func TestCAPersistence(t *testing.T) {
	dir := t.TempDir()
	ca1, err := loadOrCreateCA(dir)
	if err != nil {
		t.Fatal(err)
	}
	if _, err := os.Stat(filepath.Join(dir, "mitmproxy-ca-cert.pem")); err != nil {
		t.Errorf("agent-trusted cert file (mitmproxy-ca-cert.pem) missing: %v", err)
	}
	ca2, err := loadOrCreateCA(dir) // reload — must reuse, not regenerate
	if err != nil {
		t.Fatal(err)
	}
	if !bytes.Equal(ca1.Certificate[0], ca2.Certificate[0]) {
		t.Error("CA changed across reload — fingerprint not stable (would break boundary self-test B5)")
	}
}
