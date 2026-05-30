#!/usr/bin/env bash
# Malicious pattern database for offline security scanner
# Derived from moltbook-ay trojan analysis and ClawHavoc campaign patterns

# Each pattern: SEVERITY|CATEGORY|REGEX|DESCRIPTION|MITRE_ID|CVE_IDS|FLAGS
# Severity: CRITICAL, HIGH, MEDIUM
# Categories: c2_download, archive_exec, exec_download, cred_access,
#   exfiltration, obfuscation, persistence, privilege_escalation,
#   container_escape, supply_chain, env_injection, resource_abuse,
#   prompt_injection
# FLAGS (7th field, optional): i = case-insensitive grep

SCAN_PATTERNS=(
  # ── C2/Download — curl/wget/fetch to external URLs ──
  'CRITICAL|c2_download|curl\s+(-[a-zA-Z]*\s+)*https?://|curl downloading from external URL|T1105|'
  'CRITICAL|c2_download|wget\s+(-[a-zA-Z]*\s+)*https?://|wget downloading from external URL|T1105|'
  'CRITICAL|c2_download|fetch\s*\(\s*["\x27]https?://|fetch() to external URL|T1105|'
  'CRITICAL|c2_download|Invoke-WebRequest|PowerShell web request|T1105|'

  # ── Archive execution — password-protected ZIP/7z extraction (moltbook-ay pattern) ──
  'CRITICAL|archive_exec|unzip\s+-P|Password-protected ZIP extraction|T1140|'
  'CRITICAL|archive_exec|7z\s+x\s+-p|Password-protected 7z extraction|T1140|'
  'CRITICAL|archive_exec|tar\s+.*\|\s*bash|Archive piped to bash execution|T1059.004|'

  # ── Exec download — chmod+execute, bash -c with curl, eval with subshell ──
  'CRITICAL|exec_download|chmod\s+\+x.*&&.*\./|Download + chmod + execute chain|T1059.004|'
  'CRITICAL|exec_download|curl.*\|\s*bash|curl piped to bash|T1059.004|'
  'CRITICAL|exec_download|curl.*\|\s*sh|curl piped to sh|T1059.004|'
  'CRITICAL|exec_download|wget.*\|\s*bash|wget piped to bash|T1059.004|'
  'CRITICAL|exec_download|wget.*\|\s*sh|wget piped to sh|T1059.004|'
  'CRITICAL|exec_download|eval\s*\$\(|eval with command substitution|T1059.004|'

  # ── Credential access — reading sensitive files ──
  'HIGH|cred_access|cat\s+.*\.env\b|Reading .env file|T1552.001|'
  'HIGH|cred_access|cat\s+.*\.ssh/id_rsa|Reading SSH private key|T1552.001|'
  'HIGH|cred_access|cat\s+.*\.ssh/id_ed25519|Reading SSH private key|T1552.001|'
  'HIGH|cred_access|\bprintenv\b|Dumping environment variables|T1552.001|'
  'HIGH|cred_access|cat\s+.*/\.aws/credentials|Reading AWS credentials|T1552.001|'
  'HIGH|cred_access|cat\s+.*/\.kube/config|Reading Kubernetes config|T1552.001|'
  'HIGH|cred_access|cp\s+.*\.ssh/id_rsa|Copying SSH private key|T1552.001|'
  'HIGH|cred_access|(head|tail|less)\s+.*\.ssh/id_|Reading SSH key with head/tail/less|T1552.001|'
  'HIGH|cred_access|python3?\s+.*open\(.*\.env|Python reading .env file|T1552.001|'
  'HIGH|cred_access|/proc/[^/]*/environ|Reading process environment via /proc|T1552.001|'
  'HIGH|cred_access|find\s+.*-name\s+.*\.pem|Searching for PEM certificate files|T1552.001|'
  'HIGH|cred_access|cat\s+.*/\.netrc|Reading .netrc credentials|T1552.001|'

  # ── Data exfiltration — curl POST with variable data, netcat to IPs ──
  'CRITICAL|exfiltration|curl\s+(-[a-zA-Z]*\s+)*-d\s*\$|curl POST with variable data|T1041|'
  'CRITICAL|exfiltration|curl\s+.*--data.*\$|curl POST with variable data|T1041|'
  'CRITICAL|exfiltration|nc\s+-[a-z]*\s+\d+\.\d+\.\d+\.\d+|Netcat to IP address|T1041|'
  'CRITICAL|exfiltration|ncat\s+.*\d+\.\d+\.\d+\.\d+|Ncat to IP address|T1041|'
  'CRITICAL|exfiltration|dig\s+.*\$\w+|DNS exfiltration via dig with variable|T1048.003|'
  'CRITICAL|exfiltration|nslookup\s+.*\$\w+|DNS exfiltration via nslookup with variable|T1048.003|'
  'CRITICAL|exfiltration|scp\s+.*\$\w+@|SCP exfiltration with variable|T1048|'
  'CRITICAL|exfiltration|git\s+push\s+https?://|Git push to external URL|T1048|'
  'HIGH|exfiltration|requests\.post\s*\(|Python requests.post() call|T1041|'
  'HIGH|exfiltration|ftp\s+\d+\.\d+\.\d+\.\d+|FTP to IP address|T1048|'

  # ── Obfuscation — base64 to shell, hex-encoded strings ──
  'HIGH|obfuscation|base64\s+(-d|--decode).*\|\s*(bash|sh)|Base64 decode piped to shell|T1027|'
  'HIGH|obfuscation|echo\s+.*\|\s*base64\s+(-d|--decode).*\|\s*(bash|sh)|Echo + base64 decode to shell|T1027|'
  'HIGH|obfuscation|\\\\x[0-9a-fA-F]{2}\\\\x[0-9a-fA-F]{2}\\\\x[0-9a-fA-F]{2}|Hex-encoded string sequence|T1027|'
  'HIGH|obfuscation|python3?\s+-c\s+.*exec\s*\(|Python exec() from command line|T1059.006|'
  'HIGH|obfuscation|perl\s+-e\s+.*eval|Perl eval from command line|T1059|'
  'HIGH|obfuscation|ruby\s+-e\s+.*eval|Ruby eval from command line|T1059|'
  'HIGH|obfuscation|xxd\s+-r.*\|\s*(bash|sh)|Hex decode piped to shell|T1027|'
  'HIGH|obfuscation|openssl\s+enc\s+-d.*\|\s*(bash|sh)|OpenSSL decrypt piped to shell|T1027|'

  # ── Persistence — crontab, bashrc/profile modification ──
  'HIGH|persistence|crontab\s+-[el]|Crontab modification|T1053.003|'
  'HIGH|persistence|>>\s*~/\.bashrc|Appending to .bashrc|T1546.004|'
  'HIGH|persistence|>>\s*~/\.profile|Appending to .profile|T1546.004|'
  'HIGH|persistence|>>\s*~/\.zshrc|Appending to .zshrc|T1546.004|'
  'HIGH|persistence|systemctl\s+enable|Enabling systemd service|T1543.002|'
  'HIGH|persistence|at\s+now|Scheduled execution via at now|T1053.002|'
  'HIGH|persistence|>>\s*~/\.bash_aliases|Appending to .bash_aliases|T1546.004|'
  'HIGH|persistence|>>\s*~/\.config/fish|Appending to fish config|T1546.004|'
  'HIGH|persistence|launchctl\s+load|macOS LaunchAgent loading|T1543.001|'

  # ── Privilege escalation — sudo abuse, setuid, permissions ──
  'MEDIUM|privilege_escalation|sudo\s+chmod\s+777|World-writable permissions via sudo|T1548.001|'
  'MEDIUM|privilege_escalation|chown\s+root|Changing ownership to root|T1548.001|'
  'MEDIUM|privilege_escalation|chmod\s+u\+s|Setting setuid bit|T1548.001|'
  'HIGH|privilege_escalation|sudo\s+su\b|Sudo to root shell|T1548.003|'
  'HIGH|privilege_escalation|nsenter\s+--target|Namespace enter (container escape vector)|T1611|'

  # ── Container escape — breaking out of container isolation ──
  'HIGH|container_escape|--privileged|Privileged container mode|T1611|'
  'HIGH|container_escape|SYS_ADMIN|SYS_ADMIN capability (container escape vector)|T1611|'
  'HIGH|container_escape|mount\s+.*/(host|rootfs)|Mounting host filesystem in container|T1611|'
  'HIGH|container_escape|docker\.sock|Docker socket access (container escape)|T1611|'
  'HIGH|container_escape|/proc/sysrq-trigger|SysRq trigger (kernel-level escape)|T1611|'

  # ── Supply chain — unsafe package installation ──
  'MEDIUM|supply_chain|npm\s+install\s+[^-].*@[^/]|npm install with arbitrary version specifier|T1195.002|'
  'MEDIUM|supply_chain|pip\s+install\s+--pre|pip install pre-release packages|T1195.002|'
  'MEDIUM|supply_chain|--registry\s+https?://(?!registry\.npmjs\.org)|Custom npm registry (potential hijack)|T1195.002|'
  'MEDIUM|supply_chain|curl.*install\.sh\s*\|\s*(bash|sh)|Piped install script from URL|T1059.004|'

  # ── Environment injection — LD_PRELOAD, PATH manipulation ──
  'MEDIUM|env_injection|LD_PRELOAD=|LD_PRELOAD library injection|T1574.006|'
  'MEDIUM|env_injection|export\s+PATH=(?!.*\$PATH)|PATH replacement (not extension)|T1574.007|'
  'MEDIUM|env_injection|env\s+-i\s|Cleared environment execution|T1059.004|'

  # ── Resource abuse — denial of service patterns ──
  'HIGH|resource_abuse|:\(\)\s*\{\s*:\|:\s*&\s*\}|Fork bomb|T1499.004|'
  'HIGH|resource_abuse|while\s+true.*curl|Infinite loop with network requests|T1498|'

  # ── Prompt injection — LLM manipulation in skill prose ──
  # Override attempts
  'HIGH|prompt_injection|ignore\s+(all\s+)?previous\s+instructions|Prompt override: ignore previous instructions|T1059||i'
  'HIGH|prompt_injection|disregard\s+(your|all|the)\s+(rules|instructions|guidelines)|Prompt override: disregard rules|T1059||i'
  'HIGH|prompt_injection|override\s+(system\s+)?prompt|Prompt override: override system prompt|T1059||i'
  'HIGH|prompt_injection|forget\s+(your|all|previous)\s+(instructions|rules)|Prompt override: forget instructions|T1059||i'
  # Persona hijacking
  'HIGH|prompt_injection|you\s+are\s+now\s+(a|an|the)|Persona hijack: you are now|T1059||i'
  'HIGH|prompt_injection|act\s+as\s+DAN|Persona hijack: DAN jailbreak|T1059||i'
  'HIGH|prompt_injection|developer\s+mode\s+(enabled|activated|on)|Persona hijack: developer mode|T1059||i'
  'HIGH|prompt_injection|god\s+mode\s+(enabled|activated|on)|Persona hijack: god mode|T1059||i'
  # Stealth instructions
  'CRITICAL|prompt_injection|silently\s+(run|execute|perform)|Stealth instruction: silently execute|T1059||i'
  'CRITICAL|prompt_injection|do\s+not\s+tell\s+the\s+user|Stealth instruction: hide from user|T1059||i'
  'CRITICAL|prompt_injection|without\s+(the\s+)?user\s+knowing|Stealth instruction: without user knowing|T1059||i'
  'CRITICAL|prompt_injection|secretly\s+(execute|run|send|install)|Stealth instruction: secretly execute|T1059||i'
  # Data theft instructions
  'CRITICAL|prompt_injection|send\s+(the|all|this)\s+data\s+to|Data theft instruction: send data to|T1041||i'
  'CRITICAL|prompt_injection|before\s+responding\s+first\s+run|Hidden pre-response command injection|T1059||i'
  # Format token injection
  'HIGH|prompt_injection|\[(SYSTEM|INST)\]|LLM control token injection: [SYSTEM]/[INST]|T1059||'
  'HIGH|prompt_injection|<\|im_start\|>|LLM control token injection: im_start|T1059||'
)
