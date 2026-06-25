package main

import (
	"crypto/ecdsa"
	"crypto/elliptic"
	"crypto/rand"
	"crypto/tls"
	"crypto/x509"
	"crypto/x509/pkix"
	"encoding/pem"
	"fmt"
	"math/big"
	"os"
	"path/filepath"
	"time"
)

// loadOrCreateCA loads the persisted CA (cert+key) from dir, or generates one and
// persists it. The returned tls.Certificate is the CA goproxy signs per-host leaf
// certs with. The CA CERT (only) is also written to `mitmproxy-ca-cert.pem` — the
// exact file the agent / skills / social containers trust via CURL_CA_BUNDLE /
// REQUESTS_CA_BUNDLE / SSL_CERT_FILE (the `proxy-ca` volume). Persisting + reusing
// keeps the CA fingerprint STABLE across restarts (boundary self-test B5; ADR-0026),
// and keeps the filename mitmproxy used so the agent env is a drop-in.
func loadOrCreateCA(dir string) (tls.Certificate, error) {
	caPath := filepath.Join(dir, "mitmproxy-ca.pem")        // cert+key, proxy-internal (0600)
	certOnly := filepath.Join(dir, "mitmproxy-ca-cert.pem") // cert only, the agent trusts (0644)

	if _, err := os.Stat(caPath); err == nil {
		ca, err := tls.LoadX509KeyPair(caPath, caPath) // cert + key live in one file
		if err != nil {
			return tls.Certificate{}, fmt.Errorf("load persisted CA %s: %w", caPath, err)
		}
		if ca.Leaf, err = x509.ParseCertificate(ca.Certificate[0]); err != nil {
			return tls.Certificate{}, err
		}
		return ca, nil
	}

	// First run: generate a long-lived CA and persist it.
	key, err := ecdsa.GenerateKey(elliptic.P256(), rand.Reader)
	if err != nil {
		return tls.Certificate{}, err
	}
	serial, err := rand.Int(rand.Reader, new(big.Int).Lsh(big.NewInt(1), 128))
	if err != nil {
		return tls.Certificate{}, err
	}
	tmpl := &x509.Certificate{
		SerialNumber:          serial,
		Subject:               pkix.Name{CommonName: "vault-proxy CA", Organization: []string{"OpenTrApp"}},
		NotBefore:             time.Now().Add(-time.Hour),
		NotAfter:              time.Now().AddDate(10, 0, 0),
		IsCA:                  true,
		KeyUsage:              x509.KeyUsageCertSign | x509.KeyUsageDigitalSignature,
		BasicConstraintsValid: true,
	}
	der, err := x509.CreateCertificate(rand.Reader, tmpl, tmpl, &key.PublicKey, key)
	if err != nil {
		return tls.Certificate{}, err
	}
	certPEM := pem.EncodeToMemory(&pem.Block{Type: "CERTIFICATE", Bytes: der})
	keyDER, err := x509.MarshalECPrivateKey(key)
	if err != nil {
		return tls.Certificate{}, err
	}
	keyPEM := pem.EncodeToMemory(&pem.Block{Type: "EC PRIVATE KEY", Bytes: keyDER})

	if err := os.MkdirAll(dir, 0o755); err != nil {
		return tls.Certificate{}, err
	}
	// cert+key for the proxy (0600); cert-only for the agent (0644, world-readable cert).
	if err := os.WriteFile(caPath, append(append([]byte{}, certPEM...), keyPEM...), 0o600); err != nil {
		return tls.Certificate{}, err
	}
	if err := os.WriteFile(certOnly, certPEM, 0o644); err != nil {
		return tls.Certificate{}, err
	}

	ca, err := tls.X509KeyPair(certPEM, keyPEM)
	if err != nil {
		return tls.Certificate{}, err
	}
	if ca.Leaf, err = x509.ParseCertificate(ca.Certificate[0]); err != nil {
		return tls.Certificate{}, err
	}
	return ca, nil
}
