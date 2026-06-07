# claude-disciplined — Specification

A workflow enforcement system for Claude Code that ensures disciplined, traceable, goal-aligned software development. Consists of two binaries:

1. **`workflow`** — companion CLI for managing workflow state, artifacts, and queries
2. **`workflow-hook`** — Claude Code hook binary enforcing workflow constraints on agent tool calls

---

## Table of Contents

1. [Workflow Overview](#workflow-overview)
2. [Workflow Steps](#workflow-steps)
3. [Roles](#roles)
4. [Artifacts](#artifacts)
5. [File Structure](#file-structure)
6. [Front Matter Schemas](#front-matter-schemas)
7. [State Schema](#state-schema)
8. [Traceability](#traceability)
9. [Meetings and Consensus Protocol](#meetings-and-consensus-protocol)
10. [Transition Gates](#transition-gates)
11. [Non-Functional Requirements](#non-functional-requirements)
12. [Unresolved Disagreements](#unresolved-disagreements)
13. [Branch and Worktree Strategy](#branch-and-worktree-strategy)
14. [CLI Binary Contract](#cli-binary-contract)
15. [Hook Binary Contract](#hook-binary-contract)
16. [Validation and Linting](#validation-and-linting)
17. [ID System](#id-system)

---

## Workflow Overview

The workflow has two modes:

### Setup (once, at project start)

1. Product Brief
2. Requirements Capture
3. UX Foundations
4. Roadmap
5. Architecture
6. Tech Stack

### Execution (repeating)

**Per milestone:**

- Epic Planning — break the current milestone into epics, revisit architecture

**Per epic (epic execution sub-steps):**

- 7.1 Analysis (job stories, acceptance criteria, user flows)
- 7.2 UX Design (flows, wireframes, interaction behavior)
- 7.3 Technical Design (contracts, models, component breakdown)
- 7.4 Planning (task breakdown, sequencing, dependencies)
- 7.5 Implementation
- 7.6 Verification
- 7.7 Review (stakeholder)
- 7.8 Release
- 7.9 Retrospective

### Inner Loops

```
7.1 Analysis <-> 7.2 UX Design       (iterate until consistent)
         |
7.3 Technical Design                  (can escalate back to 7.2 if needed)
         |
7.4 Planning                          (can escalate back to 7.3 if needed — exception, not standard)
         |
7.5 Implementation <-> 7.6 Verification   (iterate until acceptance criteria met)
         ^                                    |
         +---- (changes requested) <--- 7.7 Review
         |
7.8 Release
         |
7.9 Retrospective
```

### Alignment Validation

At every level, work must trace back to and genuinely serve its upstream artifacts:

| Checkpoint | What's validated | Who validates |
|------------|-----------------|---------------|
| Roadmap definition | Milestone -> Requirement coverage | Analyst |
| Epic planning | Epic -> Milestone contribution | Analyst |
| Analysis | Story -> Epic objective, Story -> Requirement | Analyst |
| Planning | Task -> Story coverage, no gold-plating | Product Engineer, Staff Engineer |
| Verification | Implementation -> Acceptance criteria, UX specs, story intent | Product/Platform Engineer, UX Designer, Analyst |

---

## Workflow Steps

### Step 1: Product Brief

Define the product vision, goals, north star, and expected outcomes. Translates vague stakeholder input into a structured brief through an interview process.

**Sections in `product-brief.md`:**
- Vision
- North Star
- Goals (3rd-level headings, linkable via fragments)
- Expected Outcomes (3rd-level headings, linkable via fragments)

**Sign-off required:** Stakeholder approval via `workflow request-approval product-brief`.

### Step 2: Requirements Capture

Distill the product brief into concrete, high-level requirements. Uses a lightweight JTBD framework — high-level job stories that will be broken down during epic planning.

Requirements are individual files. Each links to goals/outcomes via `contributes`.

NFRs may also be proposed during this step via `workflow propose nfr`.

**Sign-off required:** Stakeholder approval.

### Step 3: UX Foundations

Establish project-wide UX constraints: design system foundations, visual language, interaction principles, metaphors, accessibility standards.

This is the UX equivalent of architecture — it sets patterns that every feature must follow.

**Sign-off required:** Stakeholder approval.

### Step 4: Roadmap

Sequence milestones that progressively fulfill requirements. The relationship between requirements and milestones is many-to-many: a milestone may touch multiple requirements, and a requirement may be fulfilled across multiple milestones.

The roadmap is a product artifact — it defines *what* and *when*, not *how*.

**Sign-off required:** Stakeholder approval.

### Step 5: Architecture

High-level system design scoped to milestone 1, with awareness of what comes later. Updated incrementally as milestones progress.

**Sign-off required:** Stakeholder approval.

### Step 6: Tech Stack

Technologies and rationale. Informed by architectural decisions.

**Sign-off required:** Stakeholder approval.

### Epic Planning (per milestone)

Break the current milestone into epics. Revisit and update architecture if needed. Identify technical/platform epics that enable product epics.

**Sign-off required:** Stakeholder approval of the epic breakdown.

### Epic Execution (per epic)

See [Inner Loops](#inner-loops) above for iteration patterns.

#### 7.1 Analysis

Define concrete job stories with acceptance criteria. Create user flows for each story. Validate alignment: does each story genuinely advance the epic's objective and trace to a requirement?

#### 7.2 UX Design

Define interaction flows, wireframes, feedback patterns for the stories. Must be consistent with UX foundations.

#### 7.3 Technical Design

Define API contracts, data models, component breakdown. Must be consistent with architecture.

#### 7.4 Planning

Break stories into tasks. Define sequencing and dependencies. Validate: do tasks cover all stories? Is there gold-plating?

#### 7.5 Implementation

Execute tasks. Product engineer builds vertical/feature work. Platform engineer builds horizontal/infrastructure work. Staff engineer available as consultant.

#### 7.6 Verification

Verify implementation against acceptance criteria, UX specs, and story intent. Must pass before stakeholder review.

#### 7.7 Review

Stakeholder validates the delivered work. Scope of change requests is limited to: acceptance criteria not met, UX specs not matched, interaction quality within agreed design. New features or scope changes become new stories or epics — they go back to analysis, not implementation.

#### 7.8 Release

Merge epic branch to main. Write release notes with learnings.

#### 7.9 Retrospective

All roles participate. Reviews:
- Deferred action items from the epic
- Unresolved disagreements
- Stakeholder feedback from review
- Process learnings

Unresolved disagreements that persist are added to the project-wide `unresolved.ndjson`.

---

## Roles

| Role | Owns | Description |
|------|------|-------------|
| **Stakeholder** | Vision, priorities, final approval | The user. Makes product decisions. |
| **Analyst** | Clarity, consistency, ubiquitous language | Bridges stakeholder and technical team. Alignment guardian. |
| **UX Designer** | User experience, interaction, visual design | Owns UX foundations and per-epic UX design. |
| **Staff Engineer** | Architecture, technical vision, system-level decisions | Oversees technical coherence. Consultant during implementation. |
| **Product Engineer** | Feature implementation (vertical) | Builds user-facing features end-to-end. |
| **Platform Engineer** | Infrastructure, shared capabilities (horizontal) | Builds the platform that product engineers build on. |

### Roles per Step

**Setup phase:**

| Step | Primary Driver | Involved | Approver |
|------|---------------|----------|----------|
| Product Brief | Analyst | Stakeholder, UX Designer | Stakeholder |
| Requirements Capture | Analyst | Stakeholder | Stakeholder |
| UX Foundations | UX Designer | Stakeholder, Analyst | Stakeholder |
| Roadmap | Analyst | Stakeholder, UX Designer, Staff Engineer | Stakeholder |
| Architecture | Staff Engineer | Stakeholder | Stakeholder |
| Tech Stack | Staff Engineer | Platform Engineer, Product Engineer | Stakeholder |

**Per milestone:**

| Step | Primary Driver | Involved | Approver |
|------|---------------|----------|----------|
| Epic Planning | Staff Engineer | Analyst, UX Designer, Platform Engineer, Product Engineer | Stakeholder |

**Per epic:**

| Step | Primary Driver | Involved |
|------|---------------|----------|
| 7.1 Analysis | Analyst | UX Designer, Product Engineer |
| 7.2 UX Design | UX Designer | Analyst, Product Engineer |
| 7.3 Technical Design | Product Engineer | Staff Engineer, Platform Engineer |
| 7.4 Planning | Product Engineer | Staff Engineer, Platform Engineer |
| 7.5 Implementation | Product Engineer (vertical), Platform Engineer (horizontal) | Staff Engineer |
| 7.6 Verification | Product Engineer, Platform Engineer | UX Designer, Analyst, Staff Engineer |
| 7.7 Review | — | Stakeholder |
| 7.8 Release | Product Engineer | Platform Engineer, Staff Engineer, Analyst, UX Designer |
| 7.9 Retrospective | — | All roles |

### Role Enforcement

The hook binary reads `agent_type` from the hook input JSON and validates it against the allowed roles for the current step. Agents not listed for a step are denied tool calls (except reading via CLI).

---

## Artifacts

### Project-level (on main branch)

| Artifact | Description |
|----------|-------------|
| `.workflow/state.yml` | Current workflow phase and step |
| `.workflow/counters.yml` | Auto-increment counters for all ID prefixes |
| `.workflow/approvals.ndjson` | Append-only stakeholder sign-off log |
| `.workflow/unresolved.ndjson` | Project-wide unresolved disagreements |
| `product-brief/product-brief.md` | Vision, goals, north star, outcomes |
| `product-brief/meetings/` | Meeting history for product brief |
| `requirements/REQ-NNN-slug.md` | Individual functional requirements |
| `requirements/NFR-NNN-slug.md` | Individual non-functional requirements |
| `requirements/meetings/` | Meeting history for requirements capture |
| `ux-foundations/ux-foundations.md` | Design system, visual language, interaction principles |
| `ux-foundations/meetings/` | Meeting history |
| `roadmap/roadmap.md` | Milestone sequencing |
| `roadmap/meetings/` | Meeting history |
| `architecture/architecture.md` | System design (living document, evolves per milestone) |
| `architecture/meetings/` | Meeting history |
| `tech-stack/tech-stack.md` | Technologies and rationale |
| `tech-stack/meetings/` | Meeting history |

### Per milestone (on main branch)

| Artifact | Description |
|----------|-------------|
| `milestones/M-NNN-slug/milestone.md` | Milestone definition, links to requirements |
| `milestones/M-NNN-slug/planning/meetings/` | Epic planning meeting history |

### Per epic (on epic branch)

| Artifact | Description |
|----------|-------------|
| `epic.md` | Epic definition, links to requirements |
| `analysis/meetings/` | Analysis meeting history |
| `analysis.md` | Final analysis artifact |
| `ux-design/meetings/` | UX design meeting history |
| `ux-design.md` | Final UX design artifact |
| `technical-design/meetings/` | Technical design meeting history |
| `technical-design.md` | Final technical design artifact |
| `planning/meetings/` | Planning meeting history |
| `plan.md` | Final plan artifact |
| `review/meetings/` | Review meeting history |
| `review.md` | Final review artifact |
| `release.md` | Release notes and learnings |
| `retrospective/meetings/` | Retrospective meeting history |
| `retrospective.md` | Retrospective summary |

### Per story (on epic branch)

| Artifact | Description |
|----------|-------------|
| `stories/S-NNN-slug/story.md` | Job story, acceptance criteria |
| `stories/S-NNN-slug/flows/F-NNN-slug.md` | User flow files |

### Per task (on epic branch)

| Artifact | Description |
|----------|-------------|
| `stories/S-NNN-slug/tasks/T-NNN-slug.md` | Task description and status |

### Per meeting

| Artifact | Description |
|----------|-------------|
| `meetings/NNN-topic-slug/notes.md` | Meeting notes |
| `meetings/NNN-topic-slug/decisions.ndjson` | Decisions made in this meeting |
| `meetings/NNN-topic-slug/actions/action-items.ndjson` | Action items log |
| `meetings/NNN-topic-slug/actions/AI-NNN-slug/action-item.md` | Action item details |
| `meetings/NNN-topic-slug/actions/AI-NNN-slug/resources/` | Research resources |
| `meetings/NNN-topic-slug/actions/AI-NNN-slug/artifacts/` | Produced artifacts (drafts, POCs, etc.) |

### Meeting notes structure

```markdown
# <Topic>

- **Date**: YYYY-MM-DD
- **Participants**: <roles>
- **Topic**: <specific topic>

## Discussion

<notes taken during the meeting>

## Decisions

<human-readable summary of decisions made — source of truth is decisions.ndjson>

## Action Items

<human-readable summary — source of truth is action-items.ndjson>

## Questions for Stakeholder

<if any>
```

---

## File Structure

```
project-root/
├── .workflow/
│   ├── state.yml
│   ├── counters.yml
│   ├── approvals.ndjson
│   └── unresolved.ndjson
│
├── product-brief/
│   ├── meetings/
│   │   └── NNN-topic-slug/
│   │       ├── notes.md
│   │       ├── decisions.ndjson
│   │       └── actions/
│   │           ├── action-items.ndjson
│   │           └── AI-NNN-slug/
│   │               ├── action-item.md
│   │               └── resources/
│   └── product-brief.md
│
├── requirements/
│   ├── meetings/
│   │   └── ...
│   ├── REQ-NNN-slug.md
│   └── NFR-NNN-slug.md
│
├── ux-foundations/
│   ├── meetings/
│   │   └── ...
│   └── ux-foundations.md
│
├── roadmap/
│   ├── meetings/
│   │   └── ...
│   └── roadmap.md
│
├── architecture/
│   ├── meetings/
│   │   └── ...
│   └── architecture.md
│
├── tech-stack/
│   ├── meetings/
│   │   └── ...
│   └── tech-stack.md
│
└── milestones/
    └── M-NNN-slug/
        ├── milestone.md
        ├── planning/
        │   └── meetings/
        │       └── ...
        └── epics/
            └── E-NNN-slug/
                ├── epic.md
                ├── analysis/
                │   └── meetings/
                │       └── ...
                ├── analysis.md
                ├── ux-design/
                │   └── meetings/
                │       └── ...
                ├── ux-design.md
                ├── technical-design/
                │   └── meetings/
                │       └── ...
                ├── technical-design.md
                ├── planning/
                │   └── meetings/
                │       └── ...
                ├── plan.md
                ├── review/
                │   └── meetings/
                │       └── ...
                ├── review.md
                ├── release.md
                ├── retrospective/
                │   └── meetings/
                │       └── ...
                ├── retrospective.md
                └── stories/
                    └── S-NNN-slug/
                        ├── story.md
                        ├── flows/
                        │   └── F-NNN-slug.md
                        └── tasks/
                            └── T-NNN-slug.md
```

---

## Front Matter Schemas

All markdown artifacts have YAML front matter. Files are immutable — never deleted, only status-transitioned.

### Product Brief

```yaml
---
status: draft | active | superseded | obsolete
superseded_by: null
created: YYYY-MM-DD
updated: YYYY-MM-DD
---
```

Goals and outcomes are 3rd-level headings within the document, linkable via markdown fragments.

### Requirement (REQ)

```yaml
---
id: REQ-0A1
status: draft | active | superseded | obsolete
superseded_by: null
contributes:
  - product-brief/product-brief.md#goal-fragment
  - product-brief/product-brief.md#outcome-fragment
created: YYYY-MM-DD
updated: YYYY-MM-DD
---
```

### Non-Functional Requirement (NFR)

```yaml
---
id: NFR-0A1
status: draft | active | superseded | obsolete
superseded_by: null
contributes:
  - product-brief/product-brief.md#goal-fragment
created: YYYY-MM-DD
updated: YYYY-MM-DD
---
```

NFRs are created via `workflow propose nfr` (always start as `draft`). They become `active` only after going through the consensus protocol in a meeting.

### Milestone

```yaml
---
id: M-0A1
status: draft | active | completed | superseded | obsolete
superseded_by: null
satisfies:
  - requirements/REQ-0A1-slug.md
  - requirements/REQ-0B2-slug.md
created: YYYY-MM-DD
updated: YYYY-MM-DD
---
```

### Epic

```yaml
---
id: E-0A1
status: draft | active | completed | superseded | obsolete
superseded_by: null
satisfies:
  - requirements/REQ-0A1-slug.md
created: YYYY-MM-DD
updated: YYYY-MM-DD
---
```

### Story

```yaml
---
id: S-0A1
status: draft | active | completed | superseded | obsolete
superseded_by: null
satisfies:
  - requirements/REQ-0A1-slug.md
nfrs:
  - requirements/NFR-0A1-slug.md
  - requirements/NFR-0B2-slug.md
created: YYYY-MM-DD
updated: YYYY-MM-DD
---
```

### User Flow

```yaml
---
id: F-0A1
type: happy-path | error | edge-case
status: active | superseded | obsolete
superseded_by: null
created: YYYY-MM-DD
updated: YYYY-MM-DD
---
```

### Task

```yaml
---
id: T-0A1
status: pending | in-progress | completed | blocked
informed_by:
  - path/to/meetings/NNN-slug/decisions.ndjson#D-0A1
blocked_reason: null
created: YYYY-MM-DD
updated: YYYY-MM-DD
---
```

### Action Item

```yaml
---
id: AI-0A1
type: research | proof-of-concept | stakeholder-question | draft | review
status: pending | in-progress | completed | discarded
timing: immediate | deferred
assignee: analyst | ux-designer | staff-engineer | product-engineer | platform-engineer
discarded_reason: null
created: YYYY-MM-DD
completed: YYYY-MM-DD
---
```

---

## State Schema

### `.workflow/state.yml` (on main branch)

```yaml
phase: setup | execution
step: product-brief | requirements | ux-foundations | roadmap | architecture | tech-stack | epic-planning
current_milestone: null | M-001
active_meeting: null | NNN-topic-slug
```

### `.workflow/state.yml` (on epic branch)

```yaml
phase: epic
current_milestone: M-001
epic: E-001
step: analysis | ux-design | technical-design | planning | implementation | verification | review | release | retrospective
active_meeting: null | NNN-topic-slug
```

> **Rationale:** `current_milestone` (not `milestone`) is used consistently across both main and epic branches. A single `State` struct handles both, avoiding conditional serialization or duplicate types.

### `.workflow/counters.yml`

```yaml
REQ: 0
NFR: 0
M: 0
E: 0
S: 0
T: 0
D: 0
AI: 0
F: 0
UR: 0
```

### `.workflow/approvals.ndjson`

```jsonl
{"timestamp":"2026-03-08T14:30:00Z","step":"product-brief","approved_by":"stakeholder"}
{"timestamp":"2026-03-08T16:45:00Z","step":"requirements","approved_by":"stakeholder"}
```

---

## Traceability

The traceability chain:

```
task -> story -> epic -> milestone -> requirement -> goal/outcome (in product brief)
```

Hierarchical relationships (parent-child) are expressed by the file system:
- Tasks inside story folders
- Stories inside epic folders
- Epics inside milestone folders

Cross-cutting relationships are expressed by markdown links in front matter:
- Requirements `contributes` to goals/outcomes
- Milestones `satisfies` requirements
- Epics `satisfies` requirements
- Stories `satisfies` requirements
- Stories reference `nfrs`
- Tasks `informed_by` decisions

Decisions reference their inputs:
- `informed_by`: action item IDs and/or other decision IDs

### CLI Queries

```bash
workflow query trace T-0A1              # full chain: task -> story -> epic -> milestone -> requirement -> goal
workflow query impact D-0A1             # all downstream work depending on this decision
workflow query rationale T-0A1          # decisions informing this task
workflow query coverage                 # orphaned artifacts, uncovered requirements/goals
workflow query goal "goal-fragment"     # all work tracing to this goal, with status
```

### Reverse Traceability

When a decision is superseded, the CLI flags all downstream work that references it. When a finding changes, the CLI identifies all decisions built on it and all tasks built on those decisions.

---

## Meetings and Consensus Protocol

### Meeting Lifecycle

```bash
workflow meeting start <topic-slug>     # only primary driver of current step; creates meeting folder
workflow meeting end                    # only primary driver, validates gates
workflow meeting list                   # list meetings in current step
```

**`meeting start`** creates the meeting folder structure (notes.md, decisions.ndjson, actions/). Only the primary driver of the current step can start or end a meeting.

**`meeting end`** validates:
1. All `immediate` action items are `completed` or `discarded`
2. All proposed decisions are in a terminal state (`agreed`, `agreed-with-reservations`, `resolved-by-owner`, `dropped`, `superseded`)
3. All non-dropped, non-superseded decisions have positions from all participants
4. `deferred` action items are acknowledged

### During a Meeting

```bash
workflow meeting contribute <message>                        # add contribution (role auto-detected)
workflow meeting read                                        # read current meeting state
workflow meeting propose-decision <summary>                  # propose a decision
workflow meeting drop-decision <id> --reason "..."           # drop a proposed decision
workflow meeting supersede-decision <id> --by <id>           # replace with new decision
workflow meeting record-decision <id>                        # finalize agreed decision
workflow meeting resolve-decision <id> --role <role> --justification "..." # step owner final call (guarded)
workflow meeting position <decision-id> agree --role <role>                 # register agree position
workflow meeting position <decision-id> disagree --role <role> --reason "..."            # required: --reason
workflow meeting position <decision-id> disagree-and-commit --role <role> --reason "..." # required: --reason
workflow meeting decision-status <decision-id>               # check consensus status
workflow meeting add-action <desc> --type <type> --assignee <role> [--immediate]
workflow meeting list-actions
workflow meeting list-decisions
```

### Decision States

```
proposed -> discussing -> agreed | agreed-with-reservations | resolved-by-owner | dropped | superseded
```

### Consensus Protocol

Every proposed decision requires a position from each meeting participant:

| Position | Meaning |
|----------|---------|
| `agree` | Supports the decision |
| `disagree` | Opposes the decision (must include `--reason`) |
| `disagree-and-commit` | Opposes but will support execution (must include `--reason`) |

**Resolution rules:**
- All `agree` -> status: `agreed`
- Mix of `agree` and `disagree-and-commit` -> status: `agreed-with-reservations`
- Any `disagree` -> decision stays in `discussing`, driver must address

**`record-decision` prerequisite:** all positions must be `agree` or `disagree-and-commit`. Fails if any `disagree` remains.

**`resolve-decision` guards (preventing authority abuse):**
1. Minimum iteration threshold reached (default: 3 rounds)
2. Every `disagree` position has a `reason`
3. Driver has addressed each disagreement (at least one response per disagreement)
4. `--justification` flag is required

Only after all guards pass can the step owner force-resolve. Status becomes `resolved-by-owner`.

### Decision Record (in `decisions.ndjson`)

```jsonl
{"id":"D-0A1","summary":"Templates exposed as REST resources","status":"agreed-with-reservations","informed_by":["AI-0A1","D-0A2"],"positions":[{"role":"staff-engineer","position":"agree"},{"role":"platform-engineer","position":"disagree-and-commit","reason":"Would prefer GraphQL but REST simpler for MVP"}],"iteration":2,"timestamp":"2026-03-08T14:30:00Z"}
```

For `resolved-by-owner` decisions:

```jsonl
{"id":"D-0B2","summary":"Use REST for MVP","status":"resolved-by-owner","resolved_by":"product-engineer","justification":"REST chosen for MVP simplicity. GraphQL revisit tracked as AI-020 for M-002.","informed_by":["AI-0A8"],"positions":[{"role":"platform-engineer","position":"disagree","reason":"GraphQL better for nested data","addressed":true}],"iteration":4,"timestamp":"2026-03-08T15:30:00Z"}
```

### Action Items (in `action-items.ndjson`)

```jsonl
{"id":"AI-0A1","type":"research","description":"Research event sourcing vs CRUD","assignee":"platform-engineer","timing":"deferred","status":"completed","created":"2026-03-08T14:30:00Z","completed":"2026-03-09T10:00:00Z"}
```

| Type | Description |
|------|-------------|
| `research` | Investigation, analysis, comparison |
| `proof-of-concept` | Technical spike or prototype |
| `stakeholder-question` | Question requiring stakeholder input |
| `draft` | Produce a document, spec, or design |
| `review` | Review an artifact or proposal |

| Timing | Description |
|--------|-------------|
| `immediate` | Must be completed before meeting ends; agent works on it while participating in the meeting |
| `deferred` | Carries to next meeting; must be completed before next meeting can start |

---

## Transition Gates

### Setup Phase

| From -> To | Gate Conditions |
|-----------|-----------------|
| (none) -> Product Brief | Project initialized |
| Product Brief -> Requirements | Product brief has goals and outcomes sections; stakeholder approved |
| Requirements -> UX Foundations | At least one requirement captured; stakeholder approved |
| UX Foundations -> Roadmap | UX foundations complete; stakeholder approved |
| Roadmap -> Architecture | At least one milestone defined; stakeholder approved |
| Architecture -> Tech Stack | Architecture document complete; stakeholder approved |
| Tech Stack -> Execution | Tech stack defined with rationale; stakeholder approved |

### Per Milestone

| From -> To | Gate Conditions |
|-----------|-----------------|
| (start) -> Epic Planning | Previous milestone completed (or first); architecture revisited |
| Epic Planning -> Epic Execution | At least one epic defined; stakeholder approved |

### Per Epic

| From -> To | Gate Conditions |
|-----------|-----------------|
| (start) -> Analysis | Epic branch/worktree created; unresolved items from previous epic reviewed |
| Analysis -> UX Design | All stories have acceptance criteria; at least one user flow per story (happy path minimum) |
| UX Design -> Technical Design | UX specs complete; consistent with UX foundations |
| Technical Design -> Planning | Contracts and models defined; consistent with architecture |
| Planning -> Implementation | All tasks defined with dependencies; no circular dependencies; all draft NFRs proposed before this point are decided |
| Implementation -> Verification | All planned tasks completed; all draft NFRs proposed during implementation are decided; newly active NFRs assigned to stories and tasks updated |
| Verification -> Review | All acceptance criteria verified; all NFR checklists pass; all tests pass |
| Review -> Release | Stakeholder approved |
| Release -> Retrospective | Branch merged to main; release notes written |
| Retrospective -> (done) | Retrospective meeting held; all unresolved disagreements either resolved or added to project-wide log; `workflow epic complete` called |

### Backward Transitions (Escalations)

Backward transitions are triggered via `workflow escalate <target-step> --reason "..."`. The reason is logged as a decision record.

| Backward Transition | Condition |
|---------------------|-----------|
| UX Design -> Analysis | Explicit reason logged in decision record |
| Technical Design -> UX Design | Explicit reason logged |
| Planning -> Technical Design | Explicit reason logged (exception, not standard) |
| Review -> Implementation | Change request references specific acceptance criteria or UX spec |
| Verification -> Implementation | Failing verification item identified |

---

## Non-Functional Requirements

NFRs are proposed via `workflow propose nfr <slug>`. They always start as `draft` and require the consensus protocol to become `active`.

### NFR Lifecycle

1. Any role proposes an NFR at any step
2. NFR is discussed in a meeting using the standard consensus protocol
3. If agreed, status transitions to `active`
4. Active NFRs are assigned to relevant stories via the `nfrs` front matter field
5. During verification, NFRs are part of the checklist

### NFR Gate Rules

- NFRs proposed during or before planning (steps 1-7.4): must be decided before implementation starts
- NFRs proposed during implementation (7.5): must be decided before transitioning to verification (7.6)
- If newly agreed NFRs affect in-progress stories, implementation loops back to address them

### Lint Rules

- `draft` NFRs cannot be referenced in story `nfrs` lists
- Only `active` NFRs count for verification checklists
- Every story must have an explicit `nfrs` field (can be empty `[]` if none apply)

---

## Unresolved Disagreements

Disagreements that persist beyond an epic's retrospective are tracked project-wide.

### File: `.workflow/unresolved.ndjson`

```jsonl
{"id":"UR-0A1","origin_decision":"D-0A1","origin_epic":"E-0A1","summary":"GraphQL vs REST for data layer","dissenter":"platform-engineer","reason":"GraphQL better for nested data","status":"open","history":[{"epic":"E-0A1","outcome":"deferred","notes":"REST worked for MVP but nested queries required workarounds"}]}
```

### Review Cadence

- **Epic start:** all open unresolved items must be reviewed (acknowledged — `deferred` is acceptable)
- **Epic retrospective:** all open items revisited with new learnings

### Possible Outcomes Per Review

| Outcome | Meaning |
|---------|---------|
| `resolved` | Disagreer conciliates based on evidence |
| `vindicated` | Evidence supports the disagreer; triggers new decision to change course |
| `deferred` | Not enough evidence yet; carry to next epic |

### CLI Commands

```bash
workflow unresolved add --decision <d-id> --epic <e-id> --summary "..." --dissenter <role> --reason "..."
workflow unresolved list [--status open]
workflow unresolved review <id> --outcome <outcome> --notes "..."
workflow unresolved history <id>
```

---

## Branch and Worktree Strategy

### Main Branch

Holds all project-level artifacts: product brief, requirements, UX foundations, roadmap, architecture, tech stack, milestones (definitions and epic breakdowns).

Main always reflects the *agreed* state of the project.

### Epic Branches

One branch per epic, using hex IDs: `M-001/E-001-slug`

Contains the epic's working artifacts: stories, UX design, technical design, plan, review, release, retrospective.

Each epic branch gets its own git worktree for isolation.

### Merge = Release

The merge of an epic branch to main is the release step (7.8). Epic isn't complete until retrospective (7.9) finishes on the branch before it is deleted.

---

## CLI Binary Contract

### Binary: `workflow`

Invoked by agents via Bash tool. Accepts content input via stdin where noted. All output is JSON to stdout. Exit 0 on success, exit 1 on error.

### State Management

```bash
workflow init                                        # initialize project
workflow status                                      # current state as JSON
workflow advance                                     # move to next step (checks gates)
workflow request-approval <step>                     # trigger macOS notification + biometric
workflow escalate <target-step> --reason "..."        # backward transition (logged as decision)
```

**`workflow request-approval`:**
1. Validates step is ready (gates pass)
2. Sends macOS notification
3. Prompts for Touch ID biometric authentication
4. If biometric passes: appends to `approvals.ndjson`, advances state
5. If fails: exit 1

**`workflow escalate`:**
1. Validates the backward transition is allowed from current step to target step
2. Logs reason as a decision record
3. Updates state to the target step
4. Exit 0 on success, exit 1 if transition not allowed

### Artifact Creation

```bash
workflow create requirement <slug>
workflow create milestone <slug>
workflow create epic <slug>                              # resolves milestone from state
workflow create story <slug>                             # resolves epic from state
workflow create task <slug> --story <story-id>           # resolves epic from state, story by ID
workflow create flow <slug> --story <story-id> --type <happy-path|error|edge-case>
workflow propose nfr <slug>
```

Context resolution:
- `epic`: reads `current_milestone` from state, finds the milestone directory
- `story`: reads `milestone` + `epic` from state (epic branch), finds the epic directory
- `task`/`flow`: resolves epic from state, then finds story directory by `--story` ID

All `create`/`propose` commands:
- Auto-increment hex ID (project-wide, 3 digits: 001-FFF)
- Create file/folder with correct front matter template
- Validate creation is allowed in current workflow state and by current role
- Exit 0 on success, exit 1 if not allowed

### Reading Artifacts

```bash
workflow read product-brief
workflow read requirement <id>
workflow read nfr <id>
workflow read milestone <id>
workflow read epic <id>
workflow read story <id>
workflow read task <id>
workflow read flow <id>
workflow read architecture
workflow read tech-stack
workflow read ux-foundations
workflow read roadmap
workflow read decision <id>
workflow read action-item <id>
workflow read meeting [<id>]                         # current meeting if no id; specific meeting by id
```

### Writing Artifact Content

Content provided via stdin.

```bash
echo "content" | workflow write requirement <id>
echo "content" | workflow write nfr <id>
echo "content" | workflow write milestone <id>
echo "content" | workflow write story <id>
echo "content" | workflow write task <id>
echo "content" | workflow write flow <id>
echo "content" | workflow write product-brief
echo "content" | workflow write analysis
echo "content" | workflow write ux-design
echo "content" | workflow write technical-design
echo "content" | workflow write plan
echo "content" | workflow write review
echo "content" | workflow write release
echo "content" | workflow write retrospective
echo "content" | workflow write architecture
echo "content" | workflow write ux-foundations
echo "content" | workflow write tech-stack
echo "content" | workflow write roadmap
```

Validates agent role and current step allow writing to that artifact.

### Status Updates

```bash
workflow update <id> --status <status>
```

### Meeting Commands

```bash
workflow meeting start <topic-slug>
workflow meeting end
workflow meeting list
workflow meeting contribute <message>
workflow meeting read
workflow meeting propose-decision <summary>
workflow meeting drop-decision <id> --reason "..."
workflow meeting supersede-decision <id> --by <id>
workflow meeting record-decision <id>
workflow meeting resolve-decision <id> --role <role> --justification "..."
workflow meeting position <decision-id> agree --role <role>
workflow meeting position <decision-id> disagree --role <role> --reason "..."
workflow meeting position <decision-id> disagree-and-commit --role <role> --reason "..."
workflow meeting decision-status <decision-id>
workflow meeting add-action <desc> --type <type> --assignee <role> [--immediate]
workflow meeting list-actions
workflow meeting list-decisions
```

### Action Item Commands

```bash
workflow action start <id>
workflow action complete <id> --summary "..."
workflow action discard <id> --reason "..."
workflow action contribute <id> <message>
workflow action add-resource <id> <path>
workflow action read <id>
workflow action list [--status <s>] [--type <t>] [--assignee <r>]
```

### Task Commands

```bash
workflow task start <id>
workflow task complete <id>
workflow task block <id> --reason "..."
workflow task unblock <id>
workflow task list [--epic <id>] [--story <id>] [--status <s>]
```

### Epic Commands

```bash
workflow epic start <id>                   # create branch/worktree
workflow epic complete <id>                # finalize after retrospective
workflow epic list [--milestone <id>] [--status <s>]
```

### Query Commands

```bash
workflow query requirements
workflow query nfrs [--status <s>]
workflow query milestones [--for <req-id>]
workflow query epics [--for <milestone-id>] [--for <req-id>]
workflow query stories [--for <epic-id>] [--for <req-id>]
workflow query tasks [--for <story-id>]
workflow query flows [--for <story-id>]
workflow query decisions [--status <s>]
workflow query actions [--type <t>] [--status <s>] [--assignee <r>]
workflow query trace <id>
workflow query impact <id>
workflow query rationale <id>
workflow query coverage
workflow query goal <fragment>
```

### Validation Commands

```bash
workflow validate                          # full lint of current step
workflow validate milestone <id>
workflow validate epic <id>
workflow validate links
workflow validate coverage
```

Output: JSON array of findings. Each finding has `severity` (`error` | `warning`) and `message`. Exit 0 if no errors, exit 1 if errors.

### Verify Commands

```bash
workflow verify story <id>
workflow verify epic <id>
workflow verify report
```

### Unresolved Disagreements

```bash
workflow unresolved add --decision <d-id> --epic <e-id> --summary "..." --dissenter <role> --reason "..."
workflow unresolved list [--status open]
workflow unresolved review <id> --outcome <outcome> --notes "..."
workflow unresolved history <id>
```

---

## Hook Binary Contract

### Binary: `workflow-hook`

Claude Code hook binary. Same I/O contract as `prevent-destructive-bash`.

### Input (stdin)

```json
{
  "session_id": "abc123",
  "hook_event_name": "PreToolUse",
  "tool_name": "Edit",
  "tool_input": {"file_path": ".workflow/state.yml", "...": "..."},
  "cwd": "/path/to/project",
  "agent_id": "agent-xyz",
  "agent_type": "analyst"
}
```

### Output

**Allow (exit 0):** silent.

**Deny (exit 2):** stderr JSON:

```json
{
  "permissionDecision": "deny",
  "reason": "Role 'product-engineer' is not allowed during step 'product-brief'. Allowed roles: analyst, ux-designer"
}
```

### Enforcement Rules

**1. File protection — deny Read/Edit/Write to workflow-controlled paths:**

All workflow artifact files are read/write protected. Agents must use the `workflow` CLI to read and write them. Protected paths include:
- `.workflow/**`
- `product-brief/**`
- `requirements/**`
- `ux-foundations/**`
- `roadmap/**`
- `architecture/**`
- `tech-stack/**`
- `milestones/**`
- Epic-level artifacts: `analysis*`, `ux-design*`, `technical-design*`, `plan.md`, `review*`, `release.md`, `retrospective*`, `epic.md`, `stories/**`

**2. Bash protection:**

Deny Bash commands that would read or modify controlled paths directly (cat, sed, echo >, etc.), except invocations of the `workflow` CLI binary itself.

**3. Role enforcement:**

Deny tool calls from agents whose `agent_type` doesn't match the allowed roles for the current workflow step. The hook reads `.workflow/state.yml` to determine the current step.

**4. Step enforcement:**

Deny artifact creation or modification that doesn't match the current step. For example, writing to `technical-design.md` during the `analysis` step.

**5. State mutation enforcement:**

Only the primary driver of the current step can call state-mutating CLI commands (`workflow advance`, `workflow escalate`, `workflow meeting start`, `workflow meeting end`, `workflow epic start`, `workflow epic complete`).

### Edge Cases

- No `.workflow/state.yml` found -> exit 0 (not a workflow-managed project)
- Missing `agent_type` in input -> exit 0 (main thread, not a role-assigned agent)
- Malformed input -> exit 0 (never block on bad input)

---

## Validation and Linting

Two-layer validation:

### Layer 1: Structural Lint (automated, by CLI)

| Rule | Type |
|------|------|
| Every story links to at least one requirement via `satisfies` | Link |
| Every epic links to at least one requirement via `satisfies` | Link |
| Every milestone links to at least one requirement via `satisfies` | Link |
| Every requirement links to at least one goal/outcome via `contributes` | Link |
| No broken links (target file or fragment exists) | Link integrity |
| No requirements without at least one milestone linking to them | Coverage |
| No goals/outcomes without at least one requirement linking to them | Coverage |
| Stories exist inside epic folders | Structure |
| Tasks exist inside story folders | Structure |
| Epics exist inside milestone folders | Structure |
| All markdown artifacts have valid front matter matching their schema | Structure |
| `draft` NFRs not referenced in story `nfrs` lists | Coherence |
| Active work doesn't depend on `obsolete`/`superseded` artifacts | Coherence |
| Superseded artifacts have `superseded_by` populated | Structure |
| Discarded action items have a `reason` | Structure |
| Every story has at least one user flow with type `happy-path` | Structure |
| Every story has an explicit `nfrs` field | Structure |
| No meeting can start if previous meeting has pending/deferred action items | Gate |
| All immediate action items completed/discarded before meeting end | Gate |
| All decisions in terminal state before meeting end | Gate |
| All positions on non-dropped decisions include reason when `disagree` or `disagree-and-commit` | Structure |

### Layer 2: Semantic Validation (agent-driven)

After lint passes, the analyst role validates that connections are *meaningful*:

- Does this story genuinely advance the requirement it links to?
- Do the tasks together actually deliver the story?
- Is the milestone's scope coherent with the requirements it claims to satisfy?

This is triggered by `workflow validate` and produces a validation report.

---

## ID System

All IDs are project-wide, 3-digit hexadecimal, auto-incremented:

| Prefix | Entity | Range |
|--------|--------|-------|
| REQ- | Functional requirement | 001-FFF |
| NFR- | Non-functional requirement | 001-FFF |
| M- | Milestone | 001-FFF |
| E- | Epic | 001-FFF |
| S- | Story | 001-FFF |
| T- | Task | 001-FFF |
| D- | Decision | 001-FFF |
| AI- | Action item | 001-FFF |
| F- | User flow | 001-FFF |
| UR- | Unresolved disagreement | 001-FFF |

Each prefix has its own counter. Maximum 4095 entities per type. IDs are globally unique within their type — T-001 refers to exactly one task across the entire project.

Counters are maintained in `.workflow/counters.yml`.
