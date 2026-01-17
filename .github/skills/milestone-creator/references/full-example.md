# Milestone Template - Full Example

This reference provides a complete example milestone for complex projects.

## Example: Database Migration Milestone

```markdown
# MILESTONE: SQLite to PostgreSQL Migration

**Status**: 🔄 IN PROGRESS
**Priority**: 🟡 HIGH
**Created**: 2026-01-17T10:00:00Z
**Estimated Duration**: 8-10 sessions (24-30 hours)
**Total Commits**: 8-12 (1-2 per phase)

---

## Executive Summary

**Objective**: Migrate the hindsight-mcp database layer from SQLite to PostgreSQL while maintaining full backward compatibility and adding connection pooling.

**Current State**: 
- SQLite database with bundled rusqlite
- Single-connection model
- Schema defined in db.rs

**The Problem**:
1. SQLite doesn't support concurrent writes efficiently
2. No connection pooling for multi-user scenarios
3. Limited scalability for large development histories

**The Solution**: 
- Abstract database layer behind a trait
- Implement PostgreSQL backend with connection pooling
- Maintain SQLite for local development/testing
- Feature flags for backend selection

---

## Success Criteria

| Metric | Target | Status |
|--------|--------|--------|
| PostgreSQL backend functional | All queries work | ⏳ Pending |
| Connection pooling | r2d2 pool working | ⏳ Pending |
| Backward compatibility | SQLite still works | ⏳ Pending |
| Test coverage | >80% for db module | ⏳ Pending |
| Migration script | Tested on sample data | ⏳ Pending |
| Performance | <10% regression on reads | ⏳ Pending |

---

## Phase Breakdown

### Phase 0: Baseline & Architecture (0.5 session)

**Status**: ✅ completed
**Completed**: 2026-01-17
**Goal**: Establish performance baselines and finalise architecture.

#### Tasks

1. **Benchmark current SQLite performance** (~50 lines)
   - Query latency for common operations
   - Memory usage under load
   - Store results in artefacts/

2. **Document architecture decision** (~100 lines)
   - Trait-based abstraction design
   - Feature flag strategy
   - Migration approach

#### Deliverables

- `artefacts/benchmark_sqlite_baseline_2026-01-17.md`
- `docs/adr/003-database-abstraction.md`

#### Validation Gate

```bash
cargo nextest run -p hindsight-mcp -- db
./scripts/benchmark_db.sh --save
```

#### Success Criteria

- [x] Baseline metrics recorded
- [x] Architecture documented and approved

**Commit**: `docs: establish database migration baseline and architecture`

---

### Phase 1: Database Trait Abstraction (1 session)

**Status**: 🔄 in-progress
**Goal**: Create abstract trait for database operations.
**Dependencies**: Phase 0

#### Tasks

1. **Define `Database` trait** (~150 lines)
   ```rust
   pub trait Database: Send + Sync {
       fn store_commit(&self, commit: &Commit) -> Result<(), DbError>;
       fn query_commits(&self, filter: &Filter) -> Result<Vec<Commit>, DbError>;
       // ... other operations
   }
   ```

2. **Refactor SQLite to implement trait** (~200 lines)
   - Extract interface from existing db.rs
   - Implement `Database` for `SqliteDb`
   - Maintain existing functionality

3. **Add feature flags** (~20 lines)
   ```toml
   [features]
   default = ["sqlite"]
   sqlite = ["rusqlite"]
   postgres = ["tokio-postgres", "r2d2"]
   ```

#### Deliverables

- `crates/hindsight-mcp/src/db/mod.rs` (refactored)
- `crates/hindsight-mcp/src/db/traits.rs` (new)
- `crates/hindsight-mcp/src/db/sqlite.rs` (new)

#### Validation Gate

```bash
cargo nextest run -p hindsight-mcp -- db
cargo build --features sqlite
```

#### Success Criteria

- [ ] Trait defined with all operations
- [ ] SQLite implementation passes existing tests
- [ ] Feature flags compile correctly

**Commit**: `refactor(db): extract Database trait abstraction`

---

### Phase 2: PostgreSQL Implementation (1.5 sessions)

**Status**: ⏳ not-started
**Goal**: Implement PostgreSQL backend with connection pooling.
**Dependencies**: Phase 1

[Continue with same structure...]

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                     hindsight-mcp                           │
├─────────────────────────────────────────────────────────────┤
│                    Database Trait                           │
│  fn store_commit() / fn query_commits() / fn migrate()     │
└──────────────┬────────────────────────┬─────────────────────┘
               │                        │
       ┌───────▼───────┐        ┌───────▼───────┐
       │   SqliteDb    │        │  PostgresDb   │
       │  (rusqlite)   │        │(tokio-postgres)│
       │               │        │    + r2d2     │
       └───────────────┘        └───────────────┘
         feature=sqlite          feature=postgres
```

---

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Schema differences | Medium | High | Comprehensive test suite |
| Connection pool tuning | Low | Medium | Configurable pool size |
| Migration data loss | Low | Critical | Backup + dry-run mode |
| Performance regression | Medium | Medium | Benchmark each phase |

---

## Notes

- Consider async database operations in future
- PostgreSQL JSONB for flexible schema evolution
- Need to document connection string configuration
```

## Template Sections Explained

### Executive Summary

Always include:
- **Objective**: One sentence, what success looks like
- **Current State**: What exists today
- **The Problem**: Why change is needed (numbered list)
- **The Solution**: How you'll solve it

### Success Criteria Table

Use these status indicators:
- ⏳ Pending - Not yet validated
- ✅ Achieved - Target met
- ❌ Failed - Target missed (needs action)
- 🔄 Partial - In progress

### Phase Structure

Each phase MUST have:
1. **Status** with emoji indicator
2. **Goal** - One sentence
3. **Dependencies** - Which phases must complete first
4. **Tasks** with line count estimates
5. **Deliverables** with file paths
6. **Validation Gate** with runnable commands
7. **Success Criteria** as checkboxes
8. **Commit** message format

### Architecture Overview

Use ASCII diagrams for clarity:
```
┌──────┐  ┌──────┐  ┌──────┐
│  A   │─▶│  B   │─▶│  C   │
└──────┘  └──────┘  └──────┘
```

Box drawing characters: `┌ ┐ └ ┘ ─ │ ┬ ┴ ├ ┤ ┼ ▶ ▼`
