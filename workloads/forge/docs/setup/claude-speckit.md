# Spec-Kit: Agentic Development Framework

> **One file to enable spec-driven, agent-optimized development in any Claude Code project.**
>
> Adapted from [GitHub Spec-Kit](https://github.com/github/spec-kit)

---

## Why Spec-Driven Development?

AI coding agents excel at pattern recognition but need unambiguous instructions. "Vibe coding" works for prototypes but fails for production software. Spec-driven development provides:

- **Clear contracts** between human intent and agent execution
- **Checkpoints** that catch misalignment early
- **Structured artifacts** agents can follow consistently
- **Measurable outcomes** that define "done"

---

## Claude Code Integration

Spec-Kit is designed to **complement, not replace** Claude Code best practices.

### How It Fits the .claude Directory Structure

```
<project>/
├── CLAUDE.md                    # ≤2.5k tokens (Boris rule) - stays lean
├── .claude/
│   ├── settings.json            # Permissions, hooks (unchanged)
│   ├── commands/                # Slash commands
│   │   ├── speckit.*.md         # ← Spec-Kit commands live here
│   │   └── your-other-cmds.md
│   └── skills/                  # Context skills (unchanged)
├── memory/
│   └── constitution.md          # ← Project principles (NOT in CLAUDE.md)
└── specs/                       # ← Feature artifacts (NOT in CLAUDE.md)
```

### What Goes Where

| Content | Location | Why |
|---------|----------|-----|
| Test/build commands | `CLAUDE.md` | Frequently needed, small |
| Code style rules | `CLAUDE.md` | Frequently needed, small |
| Project principles | `memory/constitution.md` | Loaded only when planning |
| Feature specs | `specs/*/spec.md` | Loaded only for that feature |
| Slash commands | `.claude/commands/` | Standard Claude Code location |

**Key insight**: Spec-Kit keeps CLAUDE.md lean by putting detailed specifications in `memory/` and `specs/` - loaded on-demand, not every session.

### Plan Mode Integration

Spec-Kit's workflow maps directly to Claude Code's Plan Mode:

| Spec-Kit Phase | Claude Code Mode | Why |
|----------------|------------------|-----|
| `/speckit.specify` | Plan Mode | Iterate on WHAT before HOW |
| `/speckit.clarify` | Plan Mode | Resolve ambiguities before committing |
| `/speckit.plan` | Plan Mode | Agree on architecture before coding |
| `/speckit.tasks` | Plan Mode | Review task list before execution |
| `/speckit.build` | Auto-accept | Execute agreed plan |

**Workflow**: Use `Shift+Tab` (x2) for specify/clarify/plan/tasks, then `Shift+Tab` (x1) for build.

### Verification Loops

Spec-Kit's build phase includes verification (per Boris's "most important thing"):

```markdown
## In your CLAUDE.md, add:

## Verification
- Run /speckit.build for feature implementation
- All checklist items must pass before build starts
- Tasks marked [x] only after file verified
- Constitution check validates principles
```

### Hooks Compatibility

Spec-Kit doesn't modify hooks. Your existing PostToolUse formatters work normally:

```json
{
  "hooks": {
    "PostToolUse": [{
      "matcher": "Write|Edit",
      "hooks": [{ "type": "command", "command": "prettier --write . || true" }]
    }]
  }
}
```

### Parallel Sessions

Each session can work on a different feature spec:

```bash
# Terminal 1: Working on specs/1-user-auth/
/speckit.build

# Terminal 2: Working on specs/2-payment-flow/
/speckit.specify Add payment processing

# Terminal 3: Planning next feature
/speckit.clarify
```

Use `git worktree` for parallel execution on same repo.

### Adding to CLAUDE.md (Optional)

If you want Claude to know about spec-kit without loading full docs:

```markdown
## Spec-Kit Workflow
- Non-trivial features: /speckit.specify → clarify → plan → tasks → build
- Specs in specs/, principles in memory/constitution.md
- Each phase has a gate - don't skip ahead
```

This costs ~50 tokens - well within the 2.5k budget.

---

## The Agentic Workflow

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│ CONSTITUTION│────▶│   SPECIFY   │────▶│   CLARIFY   │
│  Principles │     │  What/Why   │     │  Resolve ?s │
└─────────────┘     └─────────────┘     └──────┬──────┘
                                               │
┌─────────────┐     ┌─────────────┐     ┌──────▼──────┐
│    BUILD    │◀────│    TASKS    │◀────│    PLAN     │
│   Execute   │     │  Breakdown  │     │     How     │
└─────────────┘     └─────────────┘     └─────────────┘
```

| Phase | Command | Output | Gate |
|-------|---------|--------|------|
| 1. Constitution | `/speckit.constitution` | `memory/constitution.md` | Principles defined |
| 2. Specify | `/speckit.specify <desc>` | `specs/n-name/spec.md` | User stories complete |
| 3. Clarify | `/speckit.clarify` | Updated spec.md | Ambiguities resolved |
| 4. Plan | `/speckit.plan` | `plan.md` | Constitution check passes |
| 5. Tasks | `/speckit.tasks` | `tasks.md` | All stories have tasks |
| 6. Build | `/speckit.build` | Working code | All tasks marked [x] |

**Key principle**: Each phase has a **gate** - you don't proceed until the gate passes.

---

## Installation

### Quick Start (30 seconds)

1. Copy the `/speckit.init` command (at the end of this document) to:
   ```
   <your-project>/.claude/commands/speckit.init.md
   ```

2. In Claude Code, run:
   ```
   /speckit.init
   ```

3. Customize `memory/constitution.md` with your project's principles

4. Start your first feature:
   ```
   /speckit.specify Add user authentication with OAuth2
   ```

### What Gets Created

```
<project>/
├── memory/
│   └── constitution.md           # Project principles (your rules)
├── specs/
│   └── 1-user-auth/              # Feature directory
│       ├── spec.md               # What & Why
│       ├── plan.md               # How (technical)
│       ├── tasks.md              # Work breakdown
│       └── checklists/
│           └── requirements.md   # Quality gates
├── .speckit/
│   └── templates/                # Document templates
└── .claude/
    └── commands/
        ├── speckit.init.md
        ├── speckit.constitution.md
        ├── speckit.specify.md
        ├── speckit.clarify.md
        ├── speckit.plan.md
        ├── speckit.tasks.md
        └── speckit.build.md
```

---

## Writing Agent-Optimized Specs

### What Makes a Spec "Agent-Friendly"?

| Do This | Not This |
|---------|----------|
| "System MUST validate email format" | "Handle emails properly" |
| "Given logged-in user, When clicking logout, Then session ends" | "Users can log out" |
| "Response time < 200ms for 95th percentile" | "Should be fast" |
| "Store in PostgreSQL with UUID primary keys" | "Use a database" |
| Numbered requirements (FR-001, FR-002) | Prose paragraphs |
| File paths in tasks (`src/auth/login.ts`) | "Create the login component" |

### The Spec Hierarchy

```
User Story (P1, P2, P3...)
    └── Acceptance Scenarios (Given/When/Then)
         └── Functional Requirements (FR-001, FR-002...)
              └── Success Criteria (SC-001, SC-002...)
```

Each level adds specificity. Agents work best when they can trace from high-level story → specific testable requirement.

### Priority-Based User Stories

Stories are **independently implementable slices**:

```markdown
### User Story 1 - Basic Login (Priority: P1)

As a user, I want to log in with email/password so I can access my account.

**Independent Test**: Can be fully tested with just a login form and session.

**Acceptance Scenarios**:
1. Given valid credentials, When submitting login, Then user is authenticated
2. Given invalid password, When submitting login, Then error is shown
3. Given unregistered email, When submitting login, Then error is shown
```

**P1 = MVP**. If you only ship P1 stories, you have a working product.

---

## The Clarify Phase (Critical for Agents)

The `/speckit.clarify` command systematically finds ambiguities using this taxonomy:

| Category | What It Checks |
|----------|----------------|
| **Functional Scope** | Goals, success criteria, out-of-scope |
| **Domain & Data** | Entities, relationships, state transitions |
| **UX Flow** | User journeys, error states, edge cases |
| **Non-Functional** | Performance, scalability, security targets |
| **Integration** | External APIs, failure modes, protocols |
| **Edge Cases** | Negative scenarios, conflicts, limits |
| **Terminology** | Consistent naming, glossary |

### How Clarify Works

1. Scans spec for ambiguities (max 5 questions)
2. Asks ONE question at a time with recommendations
3. Updates spec immediately after each answer
4. Creates `## Clarifications` section with Q&A log
5. Reports coverage status when complete

**Example clarification question:**

```markdown
**Recommended:** Option B - JWT tokens are stateless and scale better for APIs

| Option | Description |
|--------|-------------|
| A | Session-based auth (server stores state) |
| B | JWT tokens (stateless, client stores token) |
| C | OAuth2 with external provider only |

Reply with option letter, "yes" for recommended, or your own answer (≤5 words).
```

---

## The Constitution (Project Principles)

The constitution defines **non-negotiable rules** that every feature must respect. Every plan includes a Constitution Check.

### Example Constitution

```markdown
# Project Constitution: MyApp

## Core Principles

### Principle 1: Security First
All user input MUST be validated. No secrets in code. OWASP Top 10 compliance required.
**Rationale**: Protect user data and company reputation.

### Principle 2: Test Coverage
All new code MUST have >80% test coverage. No PR merges with failing tests.
**Rationale**: Prevent regressions, enable confident refactoring.

### Principle 3: Performance Budget
Pages MUST load in <3s on 3G. API responses <500ms p95.
**Rationale**: User retention depends on speed.

### Principle 4: Accessibility
WCAG 2.1 AA compliance required. All interactive elements keyboard-navigable.
**Rationale**: Legal compliance and inclusive design.
```

### Constitution Check in Plans

Every plan.md includes:

```markdown
## Constitution Check

- [x] **Security First**: Input validation on all endpoints, secrets in env vars
- [x] **Test Coverage**: Unit tests for auth service, integration tests for login flow
- [ ] **Performance Budget**: Need to verify token validation <50ms
- [x] **Accessibility**: Login form has proper ARIA labels
```

---

## Task Format for Agents

Tasks must be **executable without additional context**:

```markdown
- [ ] T001 Create user model with email, password_hash, created_at in `src/models/user.ts`
- [ ] T002 [P] Add bcrypt password hashing utility in `src/utils/hash.ts`
- [ ] T003 [P] Create JWT token service in `src/services/jwt.ts`
- [ ] T004 [US1] Implement login endpoint POST /auth/login in `src/routes/auth.ts`
- [ ] T005 [US1] Add login form component in `src/components/LoginForm.tsx`
```

### Task Format Reference

```
- [ ] T### [P] [US#] Description with exact file path
```

| Marker | Meaning |
|--------|---------|
| `T###` | Sequential ID (T001, T002...) |
| `[P]` | Parallelizable (no dependencies on incomplete tasks) |
| `[US#]` | User story this task belongs to |
| File path | **Required** - agents need to know where to write |

### Phase Structure

```markdown
## Phase 1: Setup
[Project scaffolding, dependencies, config]

## Phase 2: Foundation
[Shared code that blocks all user stories]

## Phase 3: User Story 1 (P1) - [Title]
[All tasks to complete this story independently]

## Phase 4: User Story 2 (P2) - [Title]
[All tasks for next story]

## Final Phase: Polish
[Documentation, cleanup, integration tests]
```

---

## The Build Phase

`/speckit.build` executes tasks with these rules:

1. **Check all checklists** - Stop if any incomplete (unless user overrides)
2. **Load full context** - spec.md, plan.md, tasks.md, data-model.md, contracts/
3. **Verify project setup** - .gitignore, dependencies, config files
4. **Execute phase-by-phase** - Complete each phase before next
5. **Respect dependencies** - Sequential tasks in order, [P] tasks can parallelize
6. **Mark progress** - Change `[ ]` to `[x]` after each task
7. **Validate completion** - Verify against spec before reporting done

### Build Checkpoints

| Checkpoint | What's Verified |
|------------|-----------------|
| Pre-build | All checklists pass |
| Per-phase | All tasks in phase complete |
| Per-task | File created/modified as specified |
| Post-build | Implementation matches spec |

---

## Migrating Existing Projects

If you have existing specs, plans, or task lists:

```
/speckit.init migrate
```

This will:
1. Scan for existing documentation
2. Analyze structure compatibility
3. Report migration recommendations
4. Create backups before changes
5. Transform to spec-kit format

### Common Migration Patterns

| Existing Format | Migration Action |
|-----------------|------------------|
| Prose requirements | Extract to FR-001 format |
| TODO.md with checkboxes | Renumber as T001, add phases |
| JIRA-style tickets | Convert to user stories with acceptance criteria |
| RFC documents | Split into spec (what) and plan (how) |

---

## Command Reference

| Command | When to Use |
|---------|-------------|
| `/speckit.init` | First-time setup in a project |
| `/speckit.init migrate` | Port existing docs to spec-kit |
| `/speckit.constitution` | Define or update project principles |
| `/speckit.specify <desc>` | Start a new feature |
| `/speckit.clarify` | Resolve ambiguities before planning |
| `/speckit.plan` | Create technical implementation plan |
| `/speckit.tasks` | Generate task breakdown |
| `/speckit.build` | Execute the implementation |

### Workflow Shortcuts

```bash
# Full feature workflow
/speckit.specify Add payment processing
/speckit.clarify
/speckit.plan
/speckit.tasks
/speckit.build

# Quick iteration (skip clarify for small changes)
/speckit.specify Fix login button styling
/speckit.plan
/speckit.tasks
/speckit.build
```

---

## Best Practices for Agentic Development

### 1. Plan Mode for Specify/Clarify/Plan/Tasks

Use Claude Code's Plan Mode (`Shift+Tab` x2) for all phases except build. This lets you iterate on the spec/plan before Claude writes any code.

### 2. Front-Load Decisions

Make decisions in **specify** and **clarify**, not during **build**. Agents execute better than they decide.

### 3. One Story at a Time

Build P1 completely before starting P2. This catches spec issues early.

### 4. Small, Focused Specs

A spec with 3 user stories is better than one with 10. Split large features.

### 5. Explicit Over Implicit

"Use PostgreSQL 15 with pgvector extension" beats "use a vector database."

### 6. Test Criteria in Spec

If you can't write a test for a requirement, the requirement is too vague.

### 7. Review Before Build

Read the generated tasks.md before running build. It's cheaper to fix a task description than debug wrong code.

### 8. Iterate the Spec

Specs aren't sacred. If build reveals a spec problem, update the spec and re-run tasks.

### 9. Keep CLAUDE.md Lean

Spec-Kit artifacts go in `memory/` and `specs/`, NOT in CLAUDE.md. Your CLAUDE.md stays under 2.5k tokens.

### 10. Verification at Every Gate

Each phase has a gate. Don't skip `/speckit.clarify` just because you're excited to code. Ambiguities caught early save hours of rework.

---

## Troubleshooting

| Problem | Solution |
|---------|----------|
| Agent ignores requirements | Check if requirements are testable (FR-001 format) |
| Build creates wrong files | Verify file paths in tasks.md are correct |
| Constitution check fails | Update plan to address the principle |
| Too many clarification questions | Spec is too vague, add more detail upfront |
| Tasks have no file paths | Re-run `/speckit.tasks` with more specific plan |
| Agent asks for decisions during build | Need more clarification in spec |

---

## The Complete Command Files

Copy these to `.claude/commands/` in your project.

---

### speckit.init.md

```markdown
# Initialize Spec-Driven Development

Bootstrap the spec-kit workflow in this project.

## User Input

\`\`\`text
$ARGUMENTS
\`\`\`

**Modes**: `(empty)` = fresh init | `migrate` = port existing docs

## Initialization Workflow

### 1. Create Directories

\`\`\`
memory/              # Project principles
specs/               # Feature specifications
.speckit/templates/  # Document templates
\`\`\`

### 2. Create Constitution Template

Write `memory/constitution.md`:

\`\`\`markdown
# Project Constitution: [PROJECT_NAME]

**Version**: 1.0.0
**Ratification Date**: [TODAY]
**Last Amended**: [TODAY]

## Purpose

[What this project does and its primary goals]

## Core Principles

### Principle 1: [Name]

[Description - use MUST/SHOULD/MAY language]

**Rationale**: [Why this matters]

### Principle 2: [Name]

[Description]

**Rationale**: [Why this matters]

### Principle 3: [Name]

[Description]

**Rationale**: [Why this matters]

## Governance

- MAJOR version: Principle changes
- MINOR version: New principles
- PATCH version: Clarifications

All specs and plans MUST include a Constitution Check.
\`\`\`

### 3. Create Templates

Create `.speckit/templates/` with:
- `spec-template.md` - Feature specification
- `plan-template.md` - Implementation plan
- `tasks-template.md` - Task breakdown
- `checklist-template.md` - Quality gates

### 4. Create Slash Commands

Create `.claude/commands/`:
- `speckit.constitution.md`
- `speckit.specify.md`
- `speckit.clarify.md`
- `speckit.plan.md`
- `speckit.tasks.md`
- `speckit.build.md`

### 5. Report

\`\`\`
Spec-Kit initialized!

Next steps:
1. Edit memory/constitution.md with your principles
2. Run /speckit.specify <feature description>

Workflow: Constitution → Specify → Clarify → Plan → Tasks → Build
\`\`\`

## Migration Workflow

When arguments contain "migrate":

1. Scan for existing specs, plans, tasks
2. Analyze structure compatibility
3. Report findings with recommendations
4. Create backups in `archive/pre-speckit/`
5. Transform documents to spec-kit format
6. Report migration results
```

---

### speckit.constitution.md

```markdown
# Update Project Constitution

## User Input

\`\`\`text
$ARGUMENTS
\`\`\`

## Workflow

1. Load `memory/constitution.md`
2. Apply requested changes
3. Increment version (MAJOR/MINOR/PATCH)
4. Update "Last Amended" date
5. Validate principles are testable
6. Report changes and new version
```

---

### speckit.specify.md

```markdown
# Create Feature Specification

## User Input

\`\`\`text
$ARGUMENTS
\`\`\`

Use this description to create the specification.

## Workflow

1. **Generate name**: 2-4 words, lowercase with hyphens
2. **Find number**: Next available in `specs/`
3. **Create directory**: `specs/<n>-<name>/` with `checklists/`
4. **Write spec.md** with:
   - User stories (P1, P2, P3) with acceptance scenarios
   - Functional requirements (FR-001, FR-002...)
   - Success criteria (SC-001, SC-002...)
   - Edge cases
5. **Create checklist**: `checklists/requirements.md`
6. **Gate check**: All mandatory sections complete
7. **Report**: Path, any [NEEDS CLARIFICATION] markers, next step

**Guidelines**:
- Focus on WHAT and WHY, not HOW
- No implementation details
- Max 3 [NEEDS CLARIFICATION] markers
- Every requirement must be testable
```

---

### speckit.clarify.md

```markdown
# Clarify Specification

Resolve ambiguities in the current feature spec.

## User Input

\`\`\`text
$ARGUMENTS
\`\`\`

## Workflow

1. **Load spec** from current feature directory
2. **Scan for ambiguities** using taxonomy:
   - Functional scope & behavior
   - Domain & data model
   - UX flow & interactions
   - Non-functional requirements
   - Integration & dependencies
   - Edge cases & error handling
   - Terminology consistency

3. **Generate questions** (max 5):
   - Prioritize by impact on implementation
   - Each must be answerable with short answer or multiple choice
   - Provide recommended answer with reasoning

4. **Ask ONE question at a time**:
   \`\`\`
   **Recommended:** Option B - [reasoning]

   | Option | Description |
   |--------|-------------|
   | A | [description] |
   | B | [description] |
   | C | [description] |

   Reply with letter, "yes" for recommended, or own answer (≤5 words).
   \`\`\`

5. **Update spec** after each answer:
   - Add to `## Clarifications` section
   - Update relevant requirement/section
   - Save immediately

6. **Report completion**:
   - Questions asked/answered
   - Sections updated
   - Coverage status by category
   - Next step: `/speckit.plan`

**Gate**: No critical ambiguities remain (or user explicitly skips)
```

---

### speckit.plan.md

```markdown
# Create Technical Plan

## User Input

\`\`\`text
$ARGUMENTS
\`\`\`

## Workflow

1. **Load context**:
   - `memory/constitution.md`
   - Feature's `spec.md`

2. **Create plan.md** with:

   **Constitution Check** (all principles):
   \`\`\`
   - [ ] **[Principle]**: [How plan respects it]
   \`\`\`

   **Technical Context**:
   - Tech stack (language, framework, dependencies)
   - Project structure (directory tree)
   - Integration points

   **Design Decisions**:
   - Decision / Rationale / Alternatives considered

   **File Changes**:
   | File | Action | Purpose |
   |------|--------|---------|

3. **Gate check**: Constitution check passes
4. **Report**: Plan path, next step: `/speckit.tasks`
```

---

### speckit.tasks.md

```markdown
# Generate Task Breakdown

## User Input

\`\`\`text
$ARGUMENTS
\`\`\`

## Workflow

1. **Load**: spec.md, plan.md

2. **Generate tasks.md**:

   **Overview**: Total tasks, phases, parallel opportunities

   **Phase 1: Setup**
   \`\`\`
   - [ ] T001 [task with file path]
   - [ ] T002 [P] [parallelizable task with file path]
   \`\`\`

   **Phase 2: Foundation** (blocks all user stories)

   **Phase 3+: User Stories** (one phase per story, priority order)
   \`\`\`
   - [ ] T00X [US1] [task with file path]
   \`\`\`

   **Final Phase: Polish**

   **Dependencies**: What blocks what

3. **Task format**: `- [ ] T### [P] [US#] Description with file path`

4. **Gate check**: Every user story has tasks, all tasks have file paths

5. **Report**: Task count, parallel opportunities, MVP scope, next step: `/speckit.build`
```

---

### speckit.build.md

```markdown
# Execute Implementation

## User Input

\`\`\`text
$ARGUMENTS
\`\`\`

## Workflow

1. **Check checklists** in `checklists/`:
   - Count complete vs incomplete items
   - If incomplete: ask user to proceed or stop
   - If complete: continue automatically

2. **Load context**:
   - tasks.md (required)
   - plan.md (required)
   - spec.md (required)
   - data-model.md, contracts/, research.md (if exist)

3. **Verify project setup**:
   - .gitignore exists with correct patterns
   - Dependencies installed
   - Config files present

4. **Execute tasks**:
   - Phase by phase (complete each before next)
   - Sequential tasks in order
   - [P] tasks can parallelize
   - Mark `[x]` after completing each task
   - Report progress after each task

5. **Handle errors**:
   - Halt on non-parallel task failure
   - Continue parallel tasks, report failures
   - Suggest fixes

6. **Completion validation**:
   - All tasks marked [x]
   - Implementation matches spec
   - Tests pass (if applicable)

7. **Report**: Summary of completed work, any issues encountered
```

---

## Credits

- [GitHub Spec-Kit](https://github.com/github/spec-kit) - Original toolkit
- [Spec-Driven Development Blog](https://github.blog/ai-and-ml/generative-ai/spec-driven-development-with-ai-get-started-with-a-new-open-source-toolkit/)
- Workflow: Constitution → Specify → Clarify → Plan → Tasks → Build
