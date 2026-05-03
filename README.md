# Lobster-TrApp

[![CI](https://github.com/albertdobmeyer/lobster-trapp/actions/workflows/ci.yml/badge.svg)](https://github.com/albertdobmeyer/lobster-trapp/actions/workflows/ci.yml) [![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

**A safer way to run [OpenClaw](https://www.getopenclaw.ai) on your own computer.**

Lobster-TrApp installs the OpenClaw Clawbot on your machine inside a four-container sandbox, and lets you chat with it from Telegram. Every skill it loads is checked against 87 known malware patterns before it runs. The Clawbot's reasoning still goes through Anthropic's API — only its file work and tool execution stay local.

We can't promise it's safe; we can show you what we did to make it safer. The whole architecture is open source under MIT — please read it, audit it, and decide for yourself.

**Author**: [@albertdobmeyer](https://github.com/albertdobmeyer) · **Public landing page**: [lobster-trapp.com](https://lobster-trapp.com)

---

## What this is, and what it isn't

**What this is.** A desktop wrapper one open-source contributor built so that running OpenClaw on a personal computer doesn't require wiring up four containers, a proxy, a skill scanner, and a Telegram bridge by hand. There's no team behind it, no company, no funding, no business model. It's a side project shared because we hope it's useful to people who want to explore the OpenClaw ecosystem more safely than running it raw on their main machine.

**What this isn't.** A finished, audited security product. A guarantee. A replacement for thinking before you give an autonomous agent access to your machine. The OpenClaw Clawbot is genuinely powerful and genuinely hard to fully control — that's an open research problem the whole AI-safety community is working on. What we did is build the smallest cell we could think of around it, and document honestly what that cell does and doesn't catch.

## What it actually does (default Split Shell)

- **A Clawbot you talk to from Telegram on your phone.** Once paired, you message a bot, the Clawbot answers from inside its sandbox.
- **Reasoning runs on Anthropic's API**, not locally. The vault-proxy holds your `ANTHROPIC_API_KEY` and injects it per-request — the Clawbot itself never sees the key.
- **File workspace.** The Clawbot can read and summarise files you place in its sandboxed workspace. It cannot reach files outside that workspace.
- **Image processing.** Pictures you send via Telegram are processed inside the sandbox.
- **Skill scanning.** Skills the Clawbot tries to load from [ClawHub](https://www.clawhub.ai) are checked against 87 known malware patterns first. Patterns the scanner doesn't know yet can still slip through.
- **Sandboxed sandbox.** Designed to keep the Clawbot off your personal files, passwords, and SSH keys. Designed to prevent host-level installs. None of these are absolute guarantees.
- **24 startup checks** verify the sandbox topology before the Clawbot runs.

**Not enabled by default:** web browsing, web fetch, and broader tool access live at "Soft Shell" and are opt-in via CLI configuration — see [components/openclaw-vault/README.md](components/openclaw-vault/README.md). The default Split Shell is the safer setting.

## Download

Grab the latest installer for your platform from the [Releases](https://github.com/albertdobmeyer/lobster-trapp/releases) page. The setup wizard handles the rest — no terminal required.

**Requires [Podman](https://podman.io/) or [Docker](https://www.docker.com/)** (both free). The setup wizard will check for this and walk you through installation if it's missing.

**Recommended setup path:** we suggest using an AI coding assistant (such as [Claude Code](https://claude.com/claude-code)) to walk you through the install. The wizard is friendly, but if anything goes wrong on your specific machine, having an AI pair programmer next to you while you read the logs is the smoothest path.

---

<details>
<summary><strong>How it works (for the curious)</strong></summary>

The Clawbot runs inside a 4-container perimeter:

| Container | What It Does | Status |
|-----------|--------------|--------|
| **vault-agent** | Where the Clawbot runs — read-only filesystem, all capabilities dropped, custom seccomp profile | Active |
| **vault-forge** | Where skills are scanned (87 patterns + 16 prompt-injection patterns) and rebuilt safely | Active |
| **vault-proxy** | The only internet connection — holds API keys, enforces a domain allowlist, logs every request | Active |
| **vault-pioneer** | Originally meant to scan posts on the Moltbook AI-agent social network for prompt-injection patterns | **Parked** — see below |

Your API keys are held by `vault-proxy` and injected per-request; the Clawbot itself never sees them. Network traffic is filtered against the allowlist and logged. 24 startup checks verify the sandbox topology before the Clawbot is brought online. See [docs/trifecta.md](docs/trifecta.md) for the full architecture.

### About vault-pioneer

The fourth container (`vault-pioneer`) is **parked since 2026-05-03**. It was built to scan posts on [Moltbook](https://moltbook.com), an AI-agent social network, for prompt-injection patterns before they reached the Clawbot. Meta acquired Moltbook on 2026-03-10 and the public API has been intermittent since 2026-04-05. Without a stable target API the module can't reliably do its job. The container is still defined in `compose.yml` for completeness, but has no functional API to talk to. Code, docs, and threat-pattern research are preserved at [components/moltbook-pioneer/](components/moltbook-pioneer/) for whenever the network stabilises (or a successor appears). We're independent open-source researchers documenting what we observed; we can't control what corporations buy.

</details>

<details>
<summary><strong>For developers</strong></summary>

### Building from Source

All three submodules are public. No special access required.

```bash
git clone --recurse-submodules https://github.com/albertdobmeyer/lobster-trapp.git
cd lobster-trapp
cd app && npm install
npm run dev                             # Frontend dev server (Vite)
cd src-tauri && cargo build             # Rust backend
```

For a release-style desktop build, install Tauri's prerequisites for your platform and run `cd app && npm run tauri build`.

### Testing

```bash
cd app/src-tauri && cargo test --lib    # Rust backend (56 tests at v0.3.0)
cd app && npm test -- --run             # Frontend vitest (175 tests)
cd app && npx tsc --noEmit              # TypeScript strict
cd app && npx playwright test           # End-to-end (25 tests)
bash tests/orchestrator-check.sh        # Orchestration (42 checks)
podman compose up -d && podman compose down  # Container perimeter (smoke)
```

### Architecture

```
lobster-trapp/                       (this repo — desktop GUI + perimeter orchestrator)
├── components/
│   ├── openclaw-vault/              runtime (vault-agent + vault-proxy)
│   ├── clawhub-forge/               toolchain (vault-forge)
│   └── moltbook-pioneer/            network (vault-pioneer) — parked
├── app/                             Tauri 2 + React 18 desktop GUI
├── compose.yml                      4-service perimeter with network isolation
├── schemas/component.schema.json    THE CONTRACT — all manifests conform to this
└── config/orchestrator-workflows.yml  Cross-component workflow definitions
```

See [CLAUDE.md](CLAUDE.md) for the full architecture specification and contribution rules.

### Contributing

Pull requests welcome. The product is small enough that the simplest path is usually: open an issue first, sketch the change, then send a PR. Tests must stay green (`cargo test --lib`, `npm test`, `playwright test`, `orchestrator-check.sh`). The 28 banned terms in `app/e2e/user-facing.spec.ts` are enforced — don't try to work around them; if your copy needs new terminology, add it deliberately and explain why in the PR body.

</details>

---

## License

[MIT](LICENSE) — Lobster-TrApp is a gift to the community. There is no paid tier, no telemetry, no upsell. The license lets you use, modify, redistribute, and even sell derivative works of this code. All we ask in return is the attribution the license already requires (keep the copyright notice). If this is useful to you, a star on GitHub or a mention when you talk about it is the only thanks we're looking for.
