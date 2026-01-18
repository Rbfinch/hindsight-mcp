# MILESTONE: Release to crates.io

**Status**: ✅ COMPLETED
**Priority**: 🟢 DONE
**Created**: 2026-01-18T04:14:56Z
**Completed**: 2026-01-18
**Estimated Duration**: 2.5 sessions

---

## Executive Summary

**Objective**: Publish the hindsight-mcp workspace to crates.io as a public release

**Current State**: 
- Workspace has 4 crates: `hindsight-mcp`, `hindsight-git`, `hindsight-tests`, `hindsight-copilot`
- Repository is private on GitHub
- Development work is on `dev` branch
- All crates use workspace inheritance for metadata
- LICENSE.md exists at workspace root
- Repository URL in Cargo.toml points to placeholder (`your-org`)

**The Problem**: 
- Cannot publish to crates.io until workspace hygiene issues are resolved
- Internal dependencies must use version specifiers for crates.io
- Repository must be public for crates.io links to work
- Need coordinated multi-crate publishing in correct dependency order

**The Solution**:
1. Install cargo-workspaces for coordinated publishing
2. Fix workspace hygiene (repository URL, categories, documentation links)
3. Ensure internal dependencies have proper version specifiers
4. Push dev changes to main (excluding .github)
5. Make repository public
6. Dry-run publish, then publish to crates.io

---

## Success Criteria

| Metric | Target | Status |
|--------|--------|--------|
| cargo-workspaces installed | Available in PATH | ✅ v0.4.2 |
| All tests pass locally | `cargo nextest run` succeeds | ✅ 286/290 (4 intentional) |
| CI workflow passes | GitHub Actions green | ✅ Run 21106394522 |
| Repository URL correct | `https://github.com/Rbfinch/hindsight-mcp` | ✅ Updated |
| LICENSE file accessible | LICENSE.md or LICENSE at root | ✅ license-file configured |
| Internal deps have versions | All `path` deps include `version` | ✅ Added v0.1.0 |
| Dry run passes | `cargo ws publish --dry-run` succeeds | ✅ All crates packaged |
| Main branch updated | Contains latest from dev (minus .github) | ✅ Synced (92bc1e9) |
| Repository public | Visible at github.com | ✅ Public |
| Published to crates.io | All 4 crates available | ✅ v0.1.1 |

---

## Phase Breakdown

### Phase 0: Tooling and CI Validation (1 session)

**Status**: ✅ completed
**Completed**: 2026-01-18
**Goal**: Install cargo-workspaces, run all tests locally, and validate CI pipeline

#### Tasks

1. **Install cargo-workspaces** (~5 min)
   - Run `cargo install cargo-workspaces`
   - Verify installation with `cargo ws --version`

2. **Understand workspace publishing order** (~10 min)
   - Run `cargo ws list --all` to see crate dependency graph
   - Confirm publishing order: hindsight-git, hindsight-tests, hindsight-copilot, hindsight-mcp

3. **Run all tests locally** (~15 min)
   - Run `cargo nextest run --workspace -E 'not test(dummy_failing)'` to verify all tests pass
   - Note: The `dummy_failing_tests` module contains 4 intentionally failing tests used for testing hindsight's failure parsing - these are excluded from the test run
   - Run `cargo clippy --workspace` to check for lint warnings
   - Run `cargo fmt --all --check` to verify formatting

4. **Enable and run CI workflow** (~20 min)
   - Rename `.github/workflows/ci.yml.disabled` to `.github/workflows/ci.yml`
   - Commit and push to dev branch to trigger CI
   - Monitor GitHub Actions for workflow completion

5. **Verify CI success and disable workflow** (~10 min)
   - Wait for CI workflow to complete
   - If CI passes: rename `.github/workflows/ci.yml` back to `.github/workflows/ci.yml.disabled`
   - If CI fails: fix issues and re-run until passing
   - Commit the disabled workflow state

#### Validation Gate

```bash
# Local validation
cargo ws --version
cargo nextest run --workspace -E 'not test(dummy_failing)'
cargo clippy --workspace -- -D warnings
cargo fmt --all --check

# CI validation (manual check)
# Verify GitHub Actions workflow completed successfully
# Note: CI should also exclude dummy_failing_tests or expect 4 failures
```

#### Success Criteria

- [x] `cargo ws` command available (v0.4.2)
- [x] Dependency order understood (hindsight-git, hindsight-tests, hindsight-copilot -> hindsight-mcp)
- [x] All local tests pass (286 passed, 4 intentional failures in `dummy_failing_tests`)
- [x] Clippy reports no warnings
- [x] Formatting is correct
- [x] CI workflow runs successfully on GitHub Actions (run 21106394522)
- [x] CI workflow disabled after successful run

**Commit**: `ci: disable CI workflow after successful validation`

---

### Phase 1: Workspace Hygiene (1 session)

**Status**: ✅ completed
**Completed**: 2026-01-18
**Goal**: Ensure all Cargo.toml files meet crates.io publishing requirements
**Dependencies**: Phase 0 must complete successfully

#### Tasks

1. **Fix repository URL** (~5 min)
   - Update `workspace.package.repository` in root Cargo.toml
   - Change from `https://github.com/your-org/hindsight-mcp` to `https://github.com/Rbfinch/hindsight-mcp`

2. **Verify LICENSE file** (~5 min)
   - Confirm LICENSE.md exists and is MIT
   - Consider renaming to LICENSE (without .md) for crates.io convention
   - Or add `license-file = "LICENSE.md"` if keeping .md extension

3. **Add categories and keywords** (~15 min)
   - Add appropriate categories to each crate
   - Categories for hindsight-mcp: `["command-line-utilities", "development-tools"]`
   - Categories for library crates: `["development-tools", "parsing"]`

4. **Add documentation links** (~10 min)
   - Add `documentation` field pointing to docs.rs
   - Example: `documentation = "https://docs.rs/hindsight-mcp"`

5. **Verify readme paths** (~5 min)
   - Ensure each crate has a `readme` field or README.md in crate directory
   - Add crate-specific README files if missing

6. **Add homepage field** (~5 min)
   - Add `homepage = "https://github.com/Rbfinch/hindsight-mcp"` to workspace.package

#### Deliverables

- `Cargo.toml` - Updated workspace metadata
- `crates/*/Cargo.toml` - Updated crate metadata

#### Validation Gate

```bash
# Check all Cargo.toml files are valid
cargo check --workspace

# Verify metadata
cargo ws list --all
```

#### Success Criteria

- [x] Repository URL points to Rbfinch/hindsight-mcp
- [x] LICENSE file accessible (license-file = "LICENSE.md" added)
- [x] All crates have categories and keywords
- [x] Documentation links configured (docs.rs)

**Commit**: `chore: prepare workspace metadata for crates.io` (c437a17)

---

### Phase 2: Internal Dependency Versioning (0.5 session)

**Status**: ✅ completed
**Completed**: 2026-01-18
**Goal**: Ensure internal dependencies specify versions for crates.io publishing
**Dependencies**: Phase 1 must complete successfully

#### Tasks

1. **Update workspace dependency declarations** (~10 min)
   - In root Cargo.toml, add version to internal crate dependencies:
   ```toml
   [workspace.dependencies]
   hindsight-git = { path = "crates/hindsight-git", version = "0.1.0" }
   hindsight-tests = { path = "crates/hindsight-tests", version = "0.1.0" }
   hindsight-copilot = { path = "crates/hindsight-copilot", version = "0.1.0" }
   ```

2. **Verify dependency resolution** (~5 min)
   - Run `cargo check --workspace` to ensure deps resolve
   - Run `cargo tree` to verify no conflicts

#### Deliverables

- `Cargo.toml` - Internal deps with version specifiers

#### Validation Gate

```bash
cargo check --workspace
cargo tree --workspace
```

#### Success Criteria

- [x] All internal dependencies have version specifiers
- [x] Workspace builds successfully

**Commit**: `chore: add version specifiers to internal dependencies` (bb7691e)

---

### Phase 3: Dry Run Publishing (0.5 session)

**Status**: ✅ completed
**Completed**: 2026-01-18
**Goal**: Verify all crates can be packaged and pass crates.io validation
**Dependencies**: Phase 2 must complete successfully

#### Tasks

1. **Run cargo package for each crate** (~10 min)
   - Run `cargo package --list` on each crate to check included files
   - Verify no sensitive files are included

2. **Run cargo-workspaces dry run** (~10 min)
   - Run `cargo ws publish --dry-run --no-git-push`
   - Review output for any warnings or errors
   - Fix any issues identified

3. **Verify documentation builds** (~5 min)
   - Run `cargo doc --workspace --no-deps`
   - Check for documentation warnings

#### Validation Gate

```bash
cargo ws publish --dry-run --no-git-push
cargo doc --workspace --no-deps
```

#### Success Criteria

- [x] Dry run completes without errors (individual crates package successfully)
- [x] No sensitive files in packages
- [x] Documentation builds cleanly

**Commit**: `docs: update milestone Phase 3 as completed`

---

### Phase 4: Push to Main Branch (0.25 session)

**Status**: ✅ completed
**Completed**: 2026-01-18
**Goal**: Sync main branch with dev, excluding .github folder
**Dependencies**: Phase 3 must complete successfully

#### Tasks

1. **Push all dev changes to main** (~10 min)
   - Checkout main branch
   - Hard reset to dev
   - Remove .github folder from main
   - Force push main to origin

```bash
git checkout main
git reset --hard dev
git rm -r .github
git commit -m "chore: sync main with dev for release"
git push --force-with-lease origin main
git checkout dev
```

#### Validation Gate

```bash
# Verify main has latest code minus .github
git diff main dev -- ':!.github'
# Should show no differences
```

#### Success Criteria

- [x] Main branch contains all dev changes
- [x] .github folder not present on main
- [x] Main pushed to origin

**Commit**: `92bc1e9` — chore: sync main with dev for release

---

### Phase 5: Make Repository Public (0.25 session)

**Status**: ✅ completed
**Completed**: 2026-01-18
**Goal**: Change GitHub repository visibility from private to public
**Dependencies**: Phase 4 must complete successfully

#### Tasks

1. **Review repository for sensitive data** (~5 min)
   - Check git history for any secrets or sensitive files
   - Verify .gitignore excludes appropriate files

2. **Change repository visibility** (~5 min)
   - Go to GitHub repository settings
   - Navigate to "Danger Zone"
   - Click "Change repository visibility"
   - Select "Public"
   - Confirm the change

3. **Verify public access** (~2 min)
   - Access repository in incognito window
   - Verify README and LICENSE visible

#### Validation Gate

```bash
# Verify repository is accessible without auth
curl -s "https://api.github.com/repos/Rbfinch/hindsight-mcp" | grep '"private"'
# Should show: "private": false
```

#### Success Criteria

- [x] Repository visible publicly
- [x] No sensitive data exposed
- [x] README renders correctly

**Verified**: `"private": false` confirmed via GitHub API

---

### Phase 6: Publish to crates.io (0.5 session)

**Status**: ✅ completed
**Completed**: 2026-01-18
**Goal**: Publish all workspace crates to crates.io from the **main branch**
**Dependencies**: Phase 5 must complete successfully

> **Important**: Always publish from the `main` branch to ensure the published code matches what users see on GitHub.

#### Tasks

1. **Ensure you are on main branch** (~1 min)
   - Run `git checkout main` if not already on main
   - Verify with `git branch`

2. **Login to crates.io** (~2 min)
   - Run `cargo login` if not already authenticated
   - Verify with `cargo owner --list hindsight-mcp` (will fail if not published, that's OK)

3. **Final dry run** (~5 min)
   - Run `cargo ws publish --dry-run` one more time
   - Review output carefully

4. **Publish crates** (~10 min)
   - Run `cargo publish -p <crate>` for each crate in dependency order:
     1. `cargo publish -p hindsight-copilot`
     2. `cargo publish -p hindsight-git`
     3. `cargo publish -p hindsight-tests`
     4. `cargo publish -p hindsight-mcp`
   - Monitor output for any errors

5. **Verify publication** (~5 min)
   - Check each crate on crates.io:
     - https://crates.io/crates/hindsight-git
     - https://crates.io/crates/hindsight-tests
     - https://crates.io/crates/hindsight-copilot
     - https://crates.io/crates/hindsight-mcp
   - Verify `cargo install hindsight-mcp` works

6. **Return to dev branch** (~1 min)
   - Run `git checkout dev`

#### Validation Gate

```bash
# Verify crates are published
cargo search hindsight-mcp
cargo install hindsight-mcp --dry-run
```

#### Success Criteria

- [x] All 4 crates published to crates.io
- [x] Documentation available on docs.rs
- [x] `cargo install hindsight-mcp` works

**Published Version**: v0.1.1
**Crates**:
- https://crates.io/crates/hindsight-copilot
- https://crates.io/crates/hindsight-git
- https://crates.io/crates/hindsight-tests
- https://crates.io/crates/hindsight-mcp

---

## Architecture Overview

```
Publishing Order (dependency graph):

┌──────────────────┐     ┌──────────────────┐     ┌───────────────────┐
│   hindsight-git  │     │ hindsight-tests  │     │ hindsight-copilot │
│   (no internal   │     │   (no internal   │     │   (no internal    │
│    dependencies) │     │    dependencies) │     │    dependencies)  │
└────────┬─────────┘     └────────┬─────────┘     └─────────┬─────────┘
         │                        │                         │
         │                        │                         │
         └────────────────────────┼─────────────────────────┘
                                  │
                                  ▼
                       ┌──────────────────┐
                       │   hindsight-mcp  │
                       │   (depends on    │
                       │   all 3 above)   │
                       └──────────────────┘

cargo-workspaces will publish in this order automatically
```

---

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Crate name already taken | Low | High | Check availability before publishing |
| Version conflict | Low | Medium | Use dry-run to catch issues |
| Documentation fails | Low | Low | Build docs locally first |
| Repository not public | Medium | High | Verify public access before publish |
| Missing metadata | Medium | Medium | Comprehensive Phase 1 checks |

---

## Notes

### cargo-workspaces Commands

```bash
# Install
cargo install cargo-workspaces

# List crates in dependency order
cargo ws list --all

# Dry run publish
cargo ws publish --dry-run

# Publish with custom message
cargo ws publish --message "Release v0.1.0"

# Publish specific crates only
cargo ws publish --from hindsight-git
```

### crates.io Requirements

- All dependencies must be published crates (no git-only deps)
- Internal path dependencies must include version
- License must be valid SPDX identifier
- Repository URL should be accessible
- No more than 5 keywords
- Categories must be from approved list

### Post-Release Tasks

After successful release:
1. Create GitHub release with tag v0.1.0
2. Update CHANGELOG to mark v0.1.0 as released
3. Consider adding crates.io badges to README
4. Announce release on relevant channels
