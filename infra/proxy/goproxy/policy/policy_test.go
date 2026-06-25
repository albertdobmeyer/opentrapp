package policy

import (
	"net"
	"strings"
	"testing"
)

// allowlist used across the pins — mirrors infra/proxy/allowlist.txt.
func testAllowlist() map[string]bool {
	return map[string]bool{
		"api.anthropic.com":       true,
		"api.openai.com":          true,
		"api.telegram.org":        true,
		"raw.githubusercontent.com": true,
	}
}

func defaultRules() []InjectionRule {
	return []InjectionRule{
		{Match: []string{"api.anthropic.com"}, EnvVar: "ANTHROPIC_API_KEY",
			Headers: map[string]string{"x-api-key": "{key}", "anthropic-version": "2023-06-01"}},
		{Match: []string{"api.openai.com"}, EnvVar: "OPENAI_API_KEY",
			Headers: map[string]string{"Authorization": "Bearer {key}"}},
	}
}

func TestIsAllowed(t *testing.T) {
	al := testAllowlist()
	cases := []struct {
		host string
		want bool
	}{
		{"api.anthropic.com", true},
		{"API.ANTHROPIC.COM", true},          // case-insensitive
		{"foo.api.anthropic.com", true},      // subdomain
		{"api.anthropic.com.evil.com", false}, // NOT a subdomain of an allowed host
		{"example.org", false},
		{"1.1.1.1", false},   // raw IP rejected
		{"127.0.0.1", false}, // raw IP rejected
		{"10.0.0.1", false},  // raw IP rejected
		{"[::1]", false},     // IPv6 literal rejected (brackets stripped)
	}
	for _, c := range cases {
		if got := IsAllowed(c.host, al); got != c.want {
			t.Errorf("IsAllowed(%q) = %v, want %v", c.host, got, c.want)
		}
	}
}

func TestIsPrivateIP(t *testing.T) {
	priv := []string{"127.0.0.1", "10.0.0.1", "172.17.0.1", "169.254.169.254",
		"192.168.1.1", "100.64.0.1", "::1", "fe80::1", "fc00::1"}
	pub := []string{"8.8.8.8", "1.1.1.1", "140.82.121.3"}
	for _, s := range priv {
		if !IsPrivateIP(net.ParseIP(s)) {
			t.Errorf("IsPrivateIP(%s) = false, want true (private)", s)
		}
	}
	for _, s := range pub {
		if IsPrivateIP(net.ParseIP(s)) {
			t.Errorf("IsPrivateIP(%s) = true, want false (public)", s)
		}
	}
}

func TestMatchInjection(t *testing.T) {
	r := defaultRules()
	if m := MatchInjection("api.anthropic.com", r); m == nil || m.EnvVar != "ANTHROPIC_API_KEY" {
		t.Errorf("anthropic host did not match the anthropic rule: %+v", m)
	}
	if m := MatchInjection("api.openai.com", r); m == nil || m.EnvVar != "OPENAI_API_KEY" {
		t.Errorf("openai host did not match the openai rule: %+v", m)
	}
	if m := MatchInjection("foo.api.anthropic.com", r); m == nil || m.EnvVar != "ANTHROPIC_API_KEY" {
		t.Errorf("anthropic subdomain did not match: %+v", m)
	}
	if m := MatchInjection("raw.githubusercontent.com", r); m != nil {
		t.Errorf("non-provider host matched a rule: %+v", m)
	}
}

func TestRedactURL(t *testing.T) {
	in := "https://api.telegram.org/bot123456789:AAEhBOweik6ad9r-QXabcdefghij_klmn/sendMessage"
	out := RedactURL(in)
	if strings.Contains(out, "AAEhBOweik6ad9r") {
		t.Errorf("bot token not redacted: %q", out)
	}
	if !strings.Contains(out, "/bot<REDACTED_BOT_TOKEN>/sendMessage") {
		t.Errorf("redaction shape wrong: %q", out)
	}
}

func TestRedactKeys(t *testing.T) {
	body := []byte(`{"echo":"sk-secret-KEY leaked here"}`)
	out, did := RedactKeys(body, []string{"sk-secret-KEY", ""})
	if !did {
		t.Fatal("RedactKeys did not report a redaction")
	}
	if strings.Contains(string(out), "sk-secret-KEY") {
		t.Errorf("secret still present after redaction: %q", out)
	}
	if !strings.Contains(string(out), RedactedMarker) {
		t.Errorf("redaction marker missing: %q", out)
	}
	// No secrets present -> no change.
	clean := []byte(`{"ok":true}`)
	out2, did2 := RedactKeys(clean, []string{"sk-secret-KEY"})
	if did2 || string(out2) != string(clean) {
		t.Errorf("clean body changed: did=%v out=%q", did2, out2)
	}
}

func TestThresholds(t *testing.T) {
	if ExfilRequestThreshold != 1*1024*1024 {
		t.Errorf("request threshold = %d, want 1 MB", ExfilRequestThreshold)
	}
	if ExfilResponseThreshold != 10*1024*1024 {
		t.Errorf("response threshold = %d, want 10 MB", ExfilResponseThreshold)
	}
}

func TestDecideRequest(t *testing.T) {
	al := testAllowlist()
	r := defaultRules()

	// Off-allowlist -> 403, no injection.
	d := DecideRequest("example.org", al, false, 10, r)
	if !d.Block || d.Status != 403 || d.Inject != nil {
		t.Errorf("off-allowlist: %+v, want Block 403 no-inject", d)
	}

	// Allowlisted but rebinding -> 403.
	d = DecideRequest("api.anthropic.com", al, true, 10, r)
	if !d.Block || d.Status != 403 {
		t.Errorf("rebinding: %+v, want Block 403", d)
	}

	// THE ordering pin: oversized allowlisted request -> 413 BEFORE injection.
	d = DecideRequest("api.anthropic.com", al, false, ExfilRequestThreshold+1, r)
	if !d.Block || d.Status != 413 {
		t.Errorf("oversized: %+v, want Block 413", d)
	}
	if d.Inject != nil {
		t.Error("oversized blocked request must NOT carry an injection rule (key never attached to a blocked exfil)")
	}

	// Exactly at the threshold is NOT over -> allowed + injected.
	d = DecideRequest("api.anthropic.com", al, false, ExfilRequestThreshold, r)
	if d.Block {
		t.Errorf("at-threshold should not block: %+v", d)
	}
	if d.Inject == nil || d.Inject.EnvVar != "ANTHROPIC_API_KEY" {
		t.Errorf("at-threshold should inject the anthropic rule: %+v", d.Inject)
	}

	// Allowlisted non-provider -> allowed, no injection.
	d = DecideRequest("raw.githubusercontent.com", al, false, 10, r)
	if d.Block || d.Inject != nil {
		t.Errorf("non-provider: %+v, want allow no-inject", d)
	}
}
