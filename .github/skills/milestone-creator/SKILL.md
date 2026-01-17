---
name: milestone-creator
description: Create and manage development milestones for complex multi-phase projects. Use this skill when users want to plan large development efforts, create milestone documents, track phases of work, or need a structured approach to breaking down objectives into discrete, committable phases. Triggers on requests to create milestones, plan development work, or organise multi-phase implementations.
---

# Milestone Creator

Create structured development milestones that break complex objectives into discrete phases, each ending with a git commit.

## Core Concepts

- **Milestone**: A high-level development goal containing multiple phases
- **Phase**: A discrete unit of work that ends with a commit to `dev`
- **Phase Status**: `not-started` | `in-progress` | `completed` | `blocked`

## Milestone File Convention

- **Location**: `development/milestones/`
- **Naming**: `<ISO-8601-datetime>-<milestone-name>.md`
- **Example**: `2026-01-17T14-30-00Z-implement-git-parser.md`

Generate the datetime using: `date -u +"%Y-%m-%dT%H-%M-%SZ"`

## Creating a Milestone

1. Analyse the objective and identify discrete phases
2. Each phase should be completable in 1-2 sessions
3. Define clear success criteria for each phase
4. Create the milestone file using the template

## Milestone Template

Use this structure for all milestone files:

```markdown
# MILESTONE: [Title]

**Status**: 🔄 IN PROGRESS | ✅ COMPLETE
**Priority**: 🔴 CRITICAL | 🟡 HIGH | 🟢 NORMAL
**Created**: [ISO 8601 timestamp]
**Estimated Duration**: [X sessions]

---

## Executive Summary

**Objective**: [Clear statement of what this milestone achieves]

**Current State**: [What exists today]

**The Problem**: [What gap or issue this addresses]

**The Solution**: [High-level approach]

---

## Success Criteria

| Metric | Target | Status |
|--------|--------|--------|
| [Criterion 1] | [Target value] | ⏳ Pending |
| [Criterion 2] | [Target value] | ⏳ Pending |

---

## Phase Breakdown

### Phase 0: [Setup/Baseline] (X session)

**Status**: ⏳ not-started | 🔄 in-progress | ✅ completed | 🚫 blocked
**Goal**: [What this phase achieves]

#### Tasks

1. **[Task name]** (~X lines)
   - [Subtask detail]
   - [Subtask detail]

2. **[Task name]** (~X lines)
   - [Subtask detail]

#### Deliverables

- `path/to/file.rs` - [Description]
- `path/to/other.rs` - [Description]

#### Validation Gate

```bash
# Commands to verify phase completion
cargo nextest run --workspace
./scripts/fast_test.sh
```

#### Success Criteria

- [ ] [Criterion 1]
- [ ] [Criterion 2]

**Commit**: `<type>(<scope>): <description>`

---

### Phase 1: [Phase Title] (X session)

**Status**: ⏳ not-started
**Goal**: [What this phase achieves]
**Dependencies**: Phase 0

[Same structure as Phase 0]

---

## Architecture Overview

[Include diagrams or explanations of key architectural decisions]

```
┌─────────────────┐
│   Component A   │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│   Component B   │
└─────────────────┘
```

---

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| [Risk 1] | Low/Med/High | Low/Med/High | [Strategy] |

---

## Notes

[Any additional context, decisions, or follow-up items]
```

## Phase Workflow

Each phase follows this pattern:

1. Update phase status to `in-progress`
2. Complete the work (delegate to appropriate agents)
3. Run validation gates
4. Update phase status to `completed`
5. Update success criteria checkboxes
6. Call Commit agent to commit and push to `dev`
7. Begin next phase

## Phase Guidelines

### Good Phase Design

- **Atomic**: Each phase produces a working, committable state
- **Testable**: Has clear validation gates
- **Sized correctly**: 1-2 sessions (3-6 hours)
- **Independent**: Minimal dependencies on future phases

### Phase Status Updates

Before each commit, update the milestone file:

```markdown
### Phase 1: Implement Parser (1 session)

**Status**: ✅ completed
**Completed**: 2026-01-17
**Commit**: `abc1234`
```

## Integration with Agents

- **Orchestrator**: Creates milestones, tracks phase progress
- **Context/Implementation/Verification/Runtime**: Execute phase work
- **Commit**: Updates milestone status, commits, pushes to `dev`

## Estimation Guidelines

| Complexity | Phases | Duration |
|------------|--------|----------|
| Simple feature | 2-3 | 1-2 sessions |
| Medium feature | 4-6 | 3-5 sessions |
| Complex feature | 8-12 | 6-10 sessions |
| Major refactor | 10-15 | 10-15 sessions |

1 session ≈ 3 hours of focused work
