# Architecture diagrams

**Document status:** Active
**Created:** 2026-05-04
**Companion documents:** [`trifecta.md`](trifecta.md) (architecture); [`whitepaper.md`](whitepaper.md) (paper-form treatment); [`threat-model.md`](threat-model.md) (attacker model).

This document collects the visual representations of the architecture in [Mermaid](https://mermaid.js.org/) form. GitHub renders Mermaid blocks natively, so all diagrams live as code in this Markdown file with no binary asset drift. Each diagram is captioned and references the source-of-truth file (`compose.yml`, `status_aggregator.rs`, `tool-control.sh`, etc.) it visualises, so a reviewer noticing a mismatch can correct it from a single trail.

ASCII fallbacks remain in the original architecture documents ([`trifecta.md`](trifecta.md), [`whitepaper.md`](whitepaper.md)) for readers on platforms that do not render Mermaid; the Mermaid sources here are the canonical drawings.

---

## 1. Four-container perimeter topology

Source of truth: [`compose.yml`](../compose.yml).

```mermaid
flowchart TB
    subgraph HOST["Host (Tier 1 — trusted)"]
        USER[User]
        GUI["OpenTrApp GUI<br/>(Tauri 2 + Rust)"]
        COORD["Trusted CLI coordinator<br/>(Claude Code or equivalent)"]
    end

    subgraph PERIMETER["Perimeter (Tier 2 — infrastructure)"]
        AGENT["vault-agent<br/>agent runtime + Telegram gateway<br/>read-only root, dropped capabilities,<br/>narrow syscall profile, workspace-only mount"]
        FORGE["vault-forge<br/>87-pattern scanner +<br/>line classifier + CDR pipeline"]
        PIONEER["vault-pioneer<br/>(parked)"]
        PROXY["vault-proxy<br/>egress gateway, holds API credentials,<br/>domain allowlist, request log"]
    end

    subgraph EXTERNAL["External (Tier 3 surfaces)"]
        ANTHROPIC[Anthropic API]
        TELEGRAM[Telegram]
        CLAWHUB[ClawHub registry]
    end

    USER --> GUI
    USER -.-> COORD
    COORD -.-> GUI
    GUI -->|compose up/down| PERIMETER
    GUI -->|management| PROXY

    AGENT --> PROXY
    FORGE --> PROXY
    PIONEER -.-> PROXY
    AGENT <-.->|"write-only volume<br/>(forge-deliveries)"| FORGE

    PROXY --> ANTHROPIC
    PROXY --> TELEGRAM
    PROXY --> CLAWHUB

    classDef trusted fill:#e7f3ff,stroke:#1f6feb,color:#000
    classDef perim fill:#fff7d6,stroke:#9a6700,color:#000
    classDef parked fill:#f6f8fa,stroke:#bbb,color:#777,stroke-dasharray: 5 5
    classDef external fill:#f0f0f0,stroke:#777,color:#000

    class USER,GUI,COORD trusted
    class AGENT,FORGE,PROXY perim
    class PIONEER parked
    class ANTHROPIC,TELEGRAM,CLAWHUB external
```

**Reading guide.** Solid arrows are routed network paths; the dashed double-arrow between `vault-agent` and `vault-forge` is the write-only `forge-deliveries` shared volume (no routed network path exists between them). The dotted line from `vault-pioneer` indicates the parked status. The four boxes inside *Perimeter* are the four containers in `compose.yml`'s `services:` map; the four arrows from Perimeter to External enumerate the only egress destinations the proxy allowlist permits.

---

## 2. Trust tiers

Source of truth: [`trifecta.md`](trifecta.md) §2.

```mermaid
flowchart TD
    subgraph T1["TIER 1 — TRUSTED (host)"]
        direction LR
        T1A["User (issues high-level intent)"]
        T1B["Trusted CLI coordinator<br/>(Claude Code or equivalent)"]
        T1C["OpenTrApp desktop GUI"]
    end

    subgraph T2["TIER 2 — INFRASTRUCTURE (perimeter)"]
        direction LR
        T2A["OpenTrApp container orchestrator"]
        T2B["Four containers: vault-agent,<br/>vault-forge, vault-pioneer, vault-proxy"]
    end

    subgraph T3["TIER 3 — CONTAINED (inside perimeter)"]
        direction LR
        T3A[agent process]
        T3B[Telegram gateway]
        T3C[Loaded skills]
        T3D[Fetched network content]
    end

    T1 -->|"decisions / commands"| T2
    T2 -->|"mechanical enforcement<br/>(no security decisions)"| T3
    T3 -.->|"observable activity<br/>(via proxy logs, status events)"| T1

    classDef trusted fill:#e7f3ff,stroke:#1f6feb,color:#000
    classDef infra fill:#fff7d6,stroke:#9a6700,color:#000
    classDef contained fill:#fde8e8,stroke:#aa3333,color:#000

    class T1A,T1B,T1C trusted
    class T2A,T2B infra
    class T3A,T3B,T3C,T3D contained
```

**Reading guide.** Tier 1 makes decisions; Tier 2 mechanically enforces them; Tier 3 performs the work. The dotted return arrow (Tier 3 → Tier 1) is *observation*, not authorisation: Tier 3 cannot promote itself; it can only act within the boundaries Tier 2 enforces and produce activity that Tier 1 then observes.

---

## 3. Network-isolation matrix

Source of truth: `compose.yml` `networks:` section and the matrix in [`trifecta.md`](trifecta.md) §3.

```mermaid
flowchart LR
    subgraph N1["network: agent-net (internal)"]
        AGENT[vault-agent]
    end

    subgraph N2["network: forge-net (internal)"]
        FORGE[vault-forge]
    end

    subgraph N3["network: pioneer-net (internal)"]
        PIONEER[vault-pioneer]
    end

    subgraph N4["network: proxy-bridge"]
        PROXY[vault-proxy]
    end

    PROXY -- "agent-net" --> AGENT
    PROXY -- "forge-net" --> FORGE
    PROXY -- "pioneer-net" --> PIONEER
    PROXY -->|public internet| INET[Public internet]

    AGENT -.->|"NO routed path"| FORGE
    AGENT -.->|"NO routed path"| PIONEER
    AGENT <==>|"write-only volume<br/>forge-deliveries"| FORGE

    HOST[Host / GUI] --> PROXY

    classDef cont fill:#fff7d6,stroke:#9a6700,color:#000
    classDef host fill:#e7f3ff,stroke:#1f6feb,color:#000
    classDef inet fill:#f0f0f0,stroke:#777,color:#000

    class AGENT,FORGE,PIONEER,PROXY cont
    class HOST host
    class INET inet
```

**Reading guide.** Each container has its own `internal: true` network; only `vault-proxy` is dual-homed onto each. Solid arrows are the only routed paths; the dotted lines from `vault-agent` to `vault-forge` and `vault-pioneer` are emphatically *not-paths* (drawn for clarity, to make the absence visible). The `==>` line is the `forge-deliveries` shared volume — a unidirectional file-system surface, not a network path. Public-internet egress is the single arrow from `vault-proxy`; no other container has a path out.

---

## 4. Agent-skill-loading flow (the CDR pipeline)

Source of truth: [`adr/0003-content-disarm-reconstruction.md`](adr/0003-content-disarm-reconstruction.md) and [`components/openskill-forge/tools/skill-cdr.sh`](../components/openskill-forge/tools/skill-cdr.sh).

```mermaid
sequenceDiagram
    participant U as User (via GUI)
    participant K as Trusted coordinator (Karen)
    participant F as vault-forge
    participant P as vault-proxy
    participant V as forge-deliveries volume
    participant A as vault-agent

    U->>K: "Install skill X from ClawHub"
    K->>F: forge.fetch_skill(X)
    F->>P: HTTPS GET clawhub.ai/skills/X
    P-->>F: skill bundle (quarantined)

    F->>F: 87-pattern static scan
    Note over F: Reject on CRITICAL hit;<br/>otherwise continue

    F->>F: Zero-trust line classifier
    Note over F: Every line classified<br/>SAFE / SUSPICIOUS / MALICIOUS;<br/>any non-SAFE quarantines

    F->>F: Parse intent → structural model
    F->>F: Reconstruct skill from intent
    Note over F: Original artefact discarded;<br/>clean version generated;<br/>SHA-256 clearance report signed

    F->>V: Write certified skill (write-only mount)
    F-->>K: forge.scan_complete(verdict)

    K->>U: "Skill X passed; install? (Y/N)"
    U->>K: "Yes"
    K->>A: vault.install_skill(X, hash)

    A->>V: Read certified skill
    A->>A: Verify hash matches signed report
    Note over A: Reject if hash mismatch
    A-->>K: install_complete

    K->>U: "Skill X installed at Split Shell"
```

**Reading guide.** The path from ClawHub to `vault-agent` is one-way through the perimeter: `vault-proxy` mediates the egress; `vault-forge` runs the pipeline offline (no further network access during scan / classify / parse / rebuild); the `forge-deliveries` volume is the single delivery channel into `vault-agent`. The agent verifies the SHA-256 hash on every load; a side-loaded skill that bypassed forge will fail this check and be refused. The user explicit-approval gate (`"Skill X passed; install? (Y/N)"`) is the friction layer the architecture relies on for the Telegram-first, click-to-install case.

---

## 5. AssistantStatus state machine

Source of truth: [`app/src-tauri/src/status_aggregator.rs`](../app/src-tauri/src/status_aggregator.rs) (the `AssistantStatus` enum + the `evaluate()` function).

```mermaid
stateDiagram-v2
    [*] --> NotSetup

    NotSetup --> Starting: Wizard complete<br/>+ .env present
    Starting --> Recovering: 1–3 of 4 containers up
    Starting --> Ok: 4 of 4 up + key probe valid
    Recovering --> Ok: 4 of 4 up + key probe valid
    Recovering --> ErrorPerimeter: All 4 stopped<br/>(unexpectedly)

    Ok --> ErrorKey: Anthropic 401<br/>on key probe
    ErrorKey --> Ok: Key rotated +<br/>probe succeeds
    Ok --> ErrorPerimeter: All 4 stopped
    ErrorPerimeter --> Recovering: Compose up retried

    Ok --> PausedByUser: pause_perimeter()
    PausedByUser --> Ok: resume_perimeter()<br/>+ probe succeeds
    PausedByUser --> Starting: resume_perimeter()<br/>(if perimeter not yet up)

    NotSetup --> [*]: User uninstalls
    note right of NotSetup
        Initial state on first launch.
        Also reachable on .env removal.
    end note

    note right of PausedByUser
        Persists across app restarts
        via ~/.opentrapp/paused
    end note
```

**Reading guide.** Seven states. `Ok` is the steady state; `ErrorKey` and `ErrorPerimeter` are the two failure modes the user is shown directly; `PausedByUser` is the user-controlled hold that persists across application restarts via the marker file. The aggregator re-evaluates the status every 60 seconds and emits a Tauri event on transition; the React frontend's hero state-machine subscribes to this event. The full transition matrix is exercised by the Rust unit tests at [`app/src-tauri/src/status_aggregator.rs`](../app/src-tauri/src/status_aggregator.rs) (`tests::*` near line 442).

---

## How these diagrams stay accurate

Each diagram cites a source-of-truth file. When that file changes, the diagram in this document is reviewed for whether it still accurately depicts the source. Specifically:

- **Diagram 1 (topology):** updated when `compose.yml`'s `services:` map changes.
- **Diagram 2 (trust tiers):** updated when [`trifecta.md`](trifecta.md) §2 is rewritten.
- **Diagram 3 (network isolation):** updated when `compose.yml`'s `networks:` section changes.
- **Diagram 4 (CDR pipeline):** updated when [`adr/0003-content-disarm-reconstruction.md`](adr/0003-content-disarm-reconstruction.md) is amended or when the pipeline scripts change shape.
- **Diagram 5 (AssistantStatus):** updated when `status_aggregator.rs`'s `AssistantStatus` enum or the `evaluate()` function changes.

A pull-request that touches any of these source files should also touch this document if the diagram needs updating.

---

## Cross-references

- [`README.md`](../README.md) embeds Diagrams 1 and 5 directly; the others are linked from the *Architecture summary* section.
- [`trifecta.md`](trifecta.md) embeds Diagrams 1 and 3 next to their ASCII fallbacks.
- [`whitepaper.md`](whitepaper.md) §3.2 cites Diagram 1; §6 cites Diagram 4.
