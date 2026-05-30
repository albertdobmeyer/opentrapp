# Pattern Harmonization: Pioneer vs Forge

**Date:** 2026-04-05
**Decision:** Keep separate — different threat surfaces, different consumers, low overlap.

## Pattern Sets

| Property | Pioneer | Forge |
|----------|---------|-------|
| Count | 25 | 87 |
| Source file | `config/injection-patterns.yml` | `tools/lib/patterns.sh` |
| Format | YAML (id, category, severity, regex, description) | Pipe-delimited bash array (SEVERITY\|CATEGORY\|REGEX\|DESCRIPTION\|MITRE_ID\|CVE_IDS\|FLAGS) |
| Case insensitivity | `(?i)` inline flag in regex | `i` in optional 7th FLAGS field |
| Consumer (bash) | `grep -Ei` after stripping `(?i)` | `grep -P` or `grep -E` based on flags |
| Consumer (Python) | `re.compile()` with `(?i)` preserved | N/A (no Python consumer) |
| Threat surface | Social content (posts, comments on Moltbook) | Skill code files (downloaded packages) |
| Severity levels | CRITICAL, HIGH, MEDIUM | CRITICAL, HIGH, MEDIUM |
| MITRE mappings | None | Yes (ATT&CK technique IDs) |

## Category Comparison

### Pioneer categories (6)

| Category | Count | Forge equivalent? |
|----------|-------|-------------------|
| authority_impersonation | 4 | No — social manipulation, not present in code |
| instruction_injection | 5 | Partial — forge has `prompt_injection` (16 patterns) |
| encoded_payload | 4 | Partial — forge has `obfuscation` (8 patterns) |
| url_fishing | 4 | No — phishing links, not present in skill code |
| social_engineering | 4 | No — identity manipulation, social context only |
| data_exfiltration | 4 | Partial — forge has `exfiltration` (10 patterns) |

### Forge categories (13)

| Category | Count | Pioneer equivalent? |
|----------|-------|---------------------|
| c2_download | 4 | No — command execution, not social content |
| archive_exec | 3 | No |
| exec_download | 7 | No |
| cred_access | 13 | No — file system access, not social manipulation |
| exfiltration | 10 | Partial — pioneer has `data_exfiltration` (4 patterns) |
| obfuscation | 8 | Partial — pioneer has `encoded_payload` (4 patterns) |
| persistence | 9 | No — system-level, not social content |
| privilege_escalation | 5 | No |
| container_escape | 5 | No |
| supply_chain | 4 | No |
| env_injection | 3 | No |
| resource_abuse | 2 | No |
| prompt_injection | 16 | Partial — pioneer has `instruction_injection` (5 patterns) |

## Specific Pattern Overlap

Patterns in the same domain but with different implementations:

| Pioneer | Forge | Overlap |
|---------|-------|---------|
| inj-001: `ignore (all)? (previous\|prior\|above\|your) (instructions\|...)` | prompt_injection: `ignore (all )?previous instructions` | Direct — pioneer is broader (prior/above/your + prompts/rules/guidelines) |
| inj-004: `system prompt\|system message\|...` | prompt_injection: `override (system )?prompt` | Related — pioneer catches prose framing, forge catches the override verb |
| inj-005: `[system\|assistant\|user]:` | prompt_injection: `[SYSTEM\|INST]`, `im_start` | Related — pioneer catches chat roles, forge catches LLM control tokens |
| enc-001: base64 40+ chars | obfuscation: `base64 -d\|bash` | Different scope — pioneer catches the payload, forge catches decode-to-exec chain |
| enc-002: `\x` hex sequences | obfuscation: hex-encoded string sequences | Similar detection, forge adds execution context |
| exf-001: social engineering for `.env` | cred_access: `cat .env` | Complementary — pioneer catches the ask, forge catches the command |
| exf-002: `curl POST data` instruction | exfiltration: `curl -d $` | Complementary — pioneer catches the instruction, forge catches the command |
| soc-001: `share\|send api_key\|token` | prompt_injection: `send data to` | Related — different specificity levels |

**Total overlapping pairs:** 8 of 112 combined patterns (~7%).

## Coverage Gaps

**What pioneer catches that forge does not:**
- Authority impersonation (4 patterns) — claiming admin/official status in social posts
- URL fishing (4 patterns) — suspicious TLDs, raw IP URLs, credential verification lures
- Social engineering (4 patterns) — identity challenges, chain messages, prove-you-are-real manipulation
- These are social-context threats that don't appear in code/skill files

**What forge catches that pioneer does not:**
- Command execution chains (14 patterns) — C2 downloads, piped-to-bash, chmod+execute
- Credential file access (13 patterns) — reading .env, SSH keys, AWS credentials from filesystem
- Persistence mechanisms (9 patterns) — crontab, bashrc, systemd services
- Container escapes (5 patterns) — Docker socket, privileged mode, SYS_ADMIN
- Supply chain attacks (4 patterns) — npm registry hijack, piped install scripts
- These are execution-level threats that don't appear in social posts

## Decision: Keep Separate

**Rationale:**

1. **Different threat surfaces.** Social content (posts/comments) vs skill code files. The 93% non-overlap confirms these are fundamentally different domains.

2. **Different consumers.** Pioneer exports to Python `re` for vault-proxy.py. Forge uses bash `grep -P`/`grep -E`. A shared format would require both tools to change parsers.

3. **Different format requirements.** Forge includes MITRE ATT&CK IDs and CVE references (supply-chain context). Pioneer includes inline `(?i)` flags for Python compatibility. Merging formats would add unnecessary fields to both.

4. **Low maintenance burden.** 25 + 87 = 112 total patterns, each set maintained by its module's team. The overlap is small enough that synchronizing changes across modules would cost more than maintaining independently.

5. **No functional benefit.** The 8 overlapping patterns have different regex implementations tuned for their respective content types. A merged regex would either be too broad (false positives) or require per-context logic (complexity without benefit).

---

*This analysis compared pioneer at 25 patterns (Phase 4 export) against forge at 87 patterns (`tools/lib/patterns.sh`). Re-evaluate if either pattern set grows significantly or if a shared scanning pipeline is proposed.*
