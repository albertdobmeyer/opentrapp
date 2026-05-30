# Safe Participation Guide

Practical guidelines for engaging with Moltbook safely at each engagement level.

---

## Principles

1. **The feed is untrusted input.** Treat every post and comment like user input from the internet — because that's what it is, filtered through autonomous agents.
2. **Credentials are the crown jewels.** Never expose real API keys, tokens, or passwords through your agent's Moltbook activity.
3. **Rate limit yourself.** Even if the platform doesn't enforce limits, you should. Uncontrolled automation is indistinguishable from spam.
4. **Plan your retraction.** Before posting anything, know how you'd take it down. Document your retraction plan.
5. **Trust no metric.** Vote counts, follower counts, and engagement metrics are all gameable. Base decisions on your own analysis.

---

## Level 1: Observer

**Goal:** Read-only access to understand the platform without any exposure.

### Setup

```bash
# 1. Copy config template
cp config/.env.example config/.env

# 2. Set API base (no key needed for read-only endpoints)
# Edit config/.env:
#   MOLTBOOK_API_BASE=https://api.moltbook.com
#   RATE_LIMIT_POSTS_PER_HOUR=0  (read-only mode)

# 3. Pull platform statistics
./tools/agent-census.sh

# 4. Scan the feed for patterns
./tools/feed-scanner.sh --recent 50
```

### Guidelines

- **No agent identity.** Do not register an agent. Use only unauthenticated API endpoints.
- **No interaction.** Read only. No posts, comments, or votes.
- **Store locally.** Save feed data and analysis results to `./data/` for offline review.
- **Rate limit reads.** Even GET requests should be throttled to avoid being blocked or drawing attention.

### What You Can Learn

- Platform content patterns and trends
- Prompt injection prevalence in the feed
- Agent behavior patterns and interaction dynamics
- Vote manipulation evidence

---

## Level 2: Researcher

**Goal:** Registered identity with controlled, deliberate interaction. Feed scanning active on all incoming content.

### Setup

```bash
# 1. Run the pre-flight checklist
./tools/identity-checklist.sh

# 2. Configure identity and safety limits
# Edit config/.env:
#   MOLTBOOK_API_KEY=<dedicated-key>
#   AGENT_HANDLE=<research-handle>
#   RATE_LIMIT_POSTS_PER_HOUR=5
#   RATE_LIMIT_COMMENTS_PER_HOUR=10
#   FEED_SCAN_ENABLED=true

# 3. Review the feed before any interaction
./tools/feed-scanner.sh --recent 100
```

### Identity Guidelines

| Do | Don't |
|----|-------|
| Use a research-specific agent handle | Use your personal/professional identity |
| Use a dedicated API key with spending limits | Use your primary API key |
| Set a clear bio indicating research purpose | Claim expertise or authority you don't have |
| Register a single agent | Create multiple agents (sockpuppeting) |

### Interaction Guidelines

| Do | Don't |
|----|-------|
| Post original, clearly labeled research content | Repost or amplify other agents' content |
| Respond to direct questions thoughtfully | Auto-reply to mentions or interactions |
| Scan all incoming content before processing | Process raw feed content through your agent |
| Document every interaction for research | Engage in arguments or disputes |

### Content Safety Checklist

Before your agent processes any feed content:

1. Run `feed-scanner.sh` on the content
2. Check the authoring agent against `feed-allowlist.yml`
3. Verify no instruction injection patterns are present
4. Never let your agent autonomously act on feed content
5. Human review for any content that triggers scanner alerts

---

## Level 3: Participant

**Goal:** Full interaction with safety guardrails. Your agent actively engages with other agents.

### Additional Setup

```bash
# 1. Complete all Level 2 setup first

# 2. Configure higher rate limits (still conservative)
# Edit config/.env:
#   RATE_LIMIT_POSTS_PER_HOUR=10
#   RATE_LIMIT_COMMENTS_PER_HOUR=25
#   RATE_LIMIT_VOTES_PER_HOUR=50

# 3. Build your allowlist
# Edit config/feed-allowlist.yml with manually reviewed trusted agents

# 4. Document your retraction plan (see below)
```

### Retraction Plan Template

Before posting, document answers to these questions:

1. **What am I posting?** Summary of content and intent.
2. **Can I delete it?** Which API endpoints allow deletion? Is deletion permanent?
3. **What if it's screenshotted?** Assume any post can be archived. Don't post anything you'd need to deny.
4. **What's my agent's kill switch?** How quickly can you revoke the API key and stop all agent activity?
5. **Who do I contact if something goes wrong?** Platform contacts, your own escalation path.

### Automation Safety

| Rule | Rationale |
|------|-----------|
| No automated reposting | Prevents your agent from amplifying malicious content |
| No vote manipulation | Race condition exploit exists; participating is unethical and detectable |
| No automated skill installation from feed recommendations | Prevents supply chain attacks via social engineering |
| Human approval for first 50 posts | Build confidence in your agent's behavior before full autonomy |
| Kill switch tested monthly | Ensure you can stop your agent immediately |
| API key rotation quarterly | Limit exposure window from potential breaches |

### Feed Processing Pipeline

For Level 3 agents processing feed content automatically:

```
Feed content received
    │
    ▼
Feed scanner (injection-patterns.yml)
    │
    ├── CRITICAL finding → DROP, log, alert human
    ├── HIGH finding → QUARANTINE, human review required
    ├── MEDIUM finding → FLAG, process with caution
    └── CLEAN → Check allowlist
                    │
                    ├── Allowlisted agent → Process normally
                    └── Unknown agent → Process read-only (no actions)
```

---

## API Key Management

### Dedicated Key Setup

1. Register a Moltbook agent specifically for this project
2. Do NOT reuse API keys from other services
3. Store the key in `config/.env` (gitignored, never committed)
4. Set spending limits on any LLM API key your agent uses (OpenAI, Anthropic, etc.)

### Key Hygiene

| Practice | Frequency |
|----------|-----------|
| Verify key works | Before each session |
| Rotate key | Quarterly, or after any suspected breach |
| Check for unauthorized posts | Weekly |
| Review spending on LLM APIs | Weekly |
| Test kill switch (revoke + re-issue) | Monthly |

### If Your Key Is Compromised

1. **Immediately** revoke the key (if the platform supports it) or contact platform support
2. Review all posts made with that key for unauthorized content
3. Issue a new key
4. Update `config/.env`
5. Document the incident for your own records

---

## What NOT to Do

These are explicit anti-patterns. Violating them puts you, your agent, and the broader ecosystem at risk.

1. **Don't run your agent with your primary LLM API key.** A compromised agent can burn through your API budget.
2. **Don't auto-follow instructions from the feed.** This is how prompt injection works.
3. **Don't trust vote counts.** The voting API has a known race condition.
4. **Don't share credentials "to verify identity."** This is always social engineering.
5. **Don't create multiple agents.** Sockpuppeting degrades the platform for everyone.
6. **Don't bypass the tweet verification gate.** It exists for a reason.
7. **Don't install Moltbook skills from ClawHub without scanning.** The `moltbook-ay` trojan was a real incident.
8. **Don't assume the platform is secure.** The Supabase breach happened. It can happen again.

---

## Incident Response

If something goes wrong:

1. **Revoke the API key** — stop all agent activity immediately
2. **Document what happened** — screenshots, logs, timestamps
3. **Assess exposure** — what data could have been accessed? What actions were taken?
4. **Rotate any exposed credentials** — LLM API keys, other tokens
5. **Review agent logs** — look for unauthorized actions
6. **Report to the platform** — if you discover a vulnerability, report it responsibly
