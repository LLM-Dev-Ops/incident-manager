# ADR-0001: Re-enabling the LLM-Dev-Ops Ecosystem Dependencies

**Status:** Proposed
**Date:** 2026-07-27
**Decision Makers:** incident-manager maintainers, LLM-Dev-Ops ecosystem owners
**Supersedes:** the informal "temporarily disabled" note in `Cargo.toml`

## Context

`llm-incident-manager` currently declares **zero** dependencies on the LLM-Dev-Ops
ecosystem. Every one of them is commented out in the root manifest.

`/workspace/agentics-dev/incident-manager/Cargo.toml`, lines 43-50, verbatim:

```toml
# LLM-Dev-Ops Ecosystem Dependencies (Phase 2A - DISABLED for production deployment)
# NOTE: All external ecosystem dependencies are temporarily disabled due to upstream dependency issues
# These can be re-enabled once the upstream repositories fix their dependency chains
# llm-sentinel-core = { git = "https://github.com/LLM-Dev-Ops/sentinel", optional = true }
# llm-observatory-core = { git = "https://github.com/LLM-Dev-Ops/observatory", optional = true }
# llm-governance-common = { git = "https://github.com/LLM-Dev-Ops/governance-dashboard", optional = true }
# llm-analytics-hub = { git = "https://github.com/LLM-Dev-Ops/analytics-hub", optional = true }
# policy-engine-benchmarks = { git = "https://github.com/LLM-Dev-Ops/policy-engine", package = "policy-engine-benchmarks", optional = true }
```

Verification of the surrounding claims turned up three additional problems that
the comment does not mention. All three mean that **uncommenting these five
lines would not restore the integration**, and would import a fresh class of
build instability.

### Finding 1 — the `ecosystem` feature is a no-op

`Cargo.toml` lines 167-172:

```toml
[features]
default = []
# NOTE: Ecosystem feature enables integration with upstream LLM-Dev-Ops crates
# When disabled (default), stub implementations are used
# Enable with: cargo build --features ecosystem
ecosystem = []
```

The feature list is empty. The five dependencies are declared `optional = true`,
which in Cargo means they are activated only by a feature that names them (as
`"dep:llm-sentinel-core"` or equivalent). Because `ecosystem = []` names none of
them, `cargo build --features ecosystem` would compile exactly the same crate
graph as `cargo build`. The documented enable path does not work even after the
lines are uncommented.

### Finding 2 — there are no stub implementations to switch away from

The comment claims "When disabled (default), stub implementations are used."
There is no such switch. `grep -rn 'feature = "ecosystem"' src/` returns **no
matches** — not one `#[cfg(feature = "ecosystem")]` exists in the codebase. The
integration code was not gated behind the feature; it was commented out
alongside the dependencies:

- `src/integrations/upstream/sentinel_adapter.rs:179-181`
  ```rust
  // NOTE: from_sentinel requires ecosystem feature (llm_sentinel_core)
  // pub fn from_sentinel(event: &llm_sentinel_core::events::AnomalyEvent) -> Self { ... }
  ```
- `src/integrations/upstream/observatory_adapter.rs:119-121`
  ```rust
  // NOTE: from_observatory_span requires ecosystem feature (llm_observatory_core)
  // pub fn from_observatory_span(span: &llm_observatory_core::span::LlmSpan) -> UpstreamLlmSpan { ... }
  ```
- `src/integrations/upstream/analytics_adapter.rs:13-14, 67, 90, 434` — the
  `use llm_analytics_hub::{...}` imports, `convert_anomaly`, several methods,
  and the `test_outlier_type_conversion` test are all commented out.

So the dependency edges are only part of what was removed. Restoring the
manifest without restoring these call sites yields five unused crates and no
behaviour change.

### Finding 3 — the commented deps carry no version constraint

Each disabled line is a bare `{ git = "...", optional = true }` with no `tag`,
`rev`, or `branch`. Cargo resolves that to the repository's default branch at
resolution time. Re-enabling them as written reproduces the exact failure mode
that sibling repo `connector-hub` is already suffering (see
`connector-hub/docs/architecture/decisions/ADR-004-pinning-upstream-git-dependencies.md`),
where a floating `branch = "main"` dependency collided with a tag-pinned copy of
the same crate pulled transitively, putting two incompatible versions of
`llm-config-core` in one lockfile. "Once the upstream repositories fix their
dependency chains" is not a state that a floating reference can preserve — a
green build reverts to red the next time an upstream `main` moves.

### The upstream crates do exist

Every named crate is present in this workspace, so the blocker is
publication/pinning discipline, not missing code:

| Declared dependency | Defining manifest |
|---|---|
| `llm-sentinel-core` | `sentinel/crates/sentinel-core/Cargo.toml:2` |
| `llm-observatory-core` | `observatory/crates/core/Cargo.toml:2` |
| `llm-governance-common` | `governance-dashboard/libs/common/Cargo.toml:2` |
| `llm-analytics-hub` | `analytics-hub/Cargo.toml:2` |
| `policy-engine-benchmarks` | `policy-engine/policy-engine-benchmarks/Cargo.toml:2` |

### Toolchain constraint

`Cargo.toml:5` sets `rust-version = "1.84"`, and the manifest carries two
exact pins that exist solely to hold that line — `async-lock = "=3.3.0"`
(line 55, commented "3.4.x requires Rust 1.85") and `base64ct = "=1.6.0"`
(line 137, commented "1.8.x requires Rust 2024"). `async-graphql` is held at
`6.0` for the same reason (line 66). Any upstream crate admitted into the graph
must therefore build on Rust 1.84 and must not force a transitive bump past it.
This is an acceptance criterion, not a detail.

## Decision

We will re-enable the ecosystem dependencies as **tag-pinned git dependencies
behind a functioning `ecosystem` feature**, and we will restore the adapter code
under `#[cfg]` gates with real fallbacks. Concretely:

1. **Pinning mechanism: git dependencies pinned to an immutable `tag`.** Not
   crates.io, not a private registry, not `path`, not `branch`.
   - *Not crates.io:* publishing five ecosystem crates publicly is a
     governance/naming decision far larger than unblocking this repo, and
     crates.io forbids git dependencies in published crates, which would force
     the whole upstream tree to publish in lockstep first.
   - *Not a private registry:* correct long-term answer, but it requires
     registry hosting and CI credential distribution across ~55 repos. It is a
     follow-on, not a prerequisite. This ADR is deliberately compatible with it —
     migrating a `tag` pin to a registry `version` pin later is a one-line
     change per dependency.
   - *Not `path`:* path dependencies only resolve inside a checkout that has all
     sibling repos laid out identically. They break the Docker build
     (`incident-manager/Dockerfile`) and any CI that clones one repo.
   - *Not `branch`:* see Finding 3.
2. **`ecosystem` becomes a real feature** that names its dependencies via
   `dep:` syntax, so `--features ecosystem` actually changes the crate graph.
3. **Every commented-out adapter method is restored under
   `#[cfg(feature = "ecosystem")]`, with a `#[cfg(not(feature = "ecosystem"))]`
   stub of the same signature**, so both configurations compile and the default
   build keeps its current behaviour.
4. **Both configurations are built in CI.** A feature that is never compiled
   rots back into this state within a release cycle.
5. **Upstream tags are a precondition, and they are tracked as upstream work.**
   incident-manager does not merge step 1 until each upstream repo has cut a tag
   whose `cargo build` succeeds standalone on Rust 1.84.

### Per-dependency target state

```toml
llm-sentinel-core        = { git = "https://github.com/LLM-Dev-Ops/sentinel",              tag = "vX.Y.Z", optional = true }
llm-observatory-core     = { git = "https://github.com/LLM-Dev-Ops/observatory",           tag = "vX.Y.Z", optional = true }
llm-governance-common    = { git = "https://github.com/LLM-Dev-Ops/governance-dashboard",  tag = "vX.Y.Z", optional = true }
llm-analytics-hub        = { git = "https://github.com/LLM-Dev-Ops/analytics-hub",         tag = "vX.Y.Z", optional = true }
policy-engine-benchmarks = { git = "https://github.com/LLM-Dev-Ops/policy-engine", package = "policy-engine-benchmarks", tag = "vX.Y.Z", optional = true }

[features]
default   = []
ecosystem = [
  "dep:llm-sentinel-core",
  "dep:llm-observatory-core",
  "dep:llm-governance-common",
  "dep:llm-analytics-hub",
]
benchmarks = ["dep:policy-engine-benchmarks"]
```

`policy-engine-benchmarks` is separated into its own `benchmarks` feature
because it is a benchmark harness, not a runtime integration — the existing
commented `# benchmarks = ["policy-engine-benchmarks"]` on line 173 already
implies this split. Bundling it into `ecosystem` would drag benchmark-only
transitive dependencies into production builds.

### Consequence for publishing

`Cargo.toml` line 10 declares `documentation = "https://docs.rs/llm-incident-manager"`.
crates.io rejects any crate whose manifest contains a git dependency, **even an
optional one**. If publishing to crates.io is still intended, the ecosystem
dependencies must move to a registry before the next publish, or publishing must
be dropped as a goal. This ADR does not decide that; it flags it as a decision
owed. Until it is made, treat `docs.rs` in the manifest as aspirational.

## Consequences

### Positive

- `--features ecosystem` becomes truthful: it changes the dependency graph and
  activates real code.
- Builds become reproducible. A tag is immutable; an upstream force-push to
  `main` can no longer turn a green build red without a deliberate commit here.
- The default build stays exactly as it is today — zero ecosystem dependencies,
  no deployment risk — so this work does not gate production releases.
- The diamond-dependency failure already observed in `connector-hub` cannot
  occur here by construction, because every edge is pinned.
- Migration to a private registry later is mechanical.

### Negative

- Five upstream repos must cut tags before step 1 can merge. incident-manager
  cannot unilaterally complete this; the critical path runs through other teams.
- Tag pins do not auto-update. Security fixes upstream require an explicit bump
  PR here. This is the intended trade — visible staleness beats invisible
  breakage — but it is real recurring cost.
- CI build time roughly doubles for the crates that gain a second feature
  configuration.
- Restoring the adapter code is not a revert. The commented-out bodies reference
  upstream type paths (`llm_observatory_core::span::LlmSpan`,
  `llm_analytics_hub::analytics::anomaly::DetectorStats`) that may have moved
  since they were disabled. Expect real porting work, not uncommenting.

### Neutral

- Two compiled configurations mean stub and real implementations can drift in
  behaviour even while both compile. Step 8 addresses this with shared
  trait-level tests, but drift remains possible.

## Implementation Plan

1. **Establish upstream tags (blocking, external).** For each of `sentinel`,
   `observatory`, `governance-dashboard`, `analytics-hub`, and `policy-engine`,
   open an issue requesting a semver tag whose tree satisfies: builds standalone
   with `cargo build --locked` on Rust 1.84; declares no floating `branch = `
   git dependencies of its own (a tag that transitively floats is not a pin);
   and forces no transitive dependency requiring Rust > 1.84. Record the
   resulting tag names.
2. **Audit for the `connector-hub` diamond.** Before pinning, run
   `cargo tree --duplicates` against a scratch branch with all five deps enabled.
   If two upstream crates disagree on a shared transitive version — the exact
   defect found in `connector-hub`'s lockfile — resolve it upstream first. Do
   not paper over it with `[patch]`.
3. **Restore the manifest** — replace lines 43-50 with the tag-pinned block
   above and rewrite the `[features]` section per the target state. One commit,
   no code changes, so the diff is reviewable in isolation.
4. **Verify the feature actually switches.** Confirm
   `cargo tree --features ecosystem` lists all four runtime crates and
   `cargo tree` lists none.
5. **Restore `sentinel_adapter.rs`** — reinstate `from_sentinel` under
   `#[cfg(feature = "ecosystem")]`, add a `#[cfg(not(feature = "ecosystem"))]`
   stub with an identical signature. Repeat for `observatory_adapter.rs`
   (`from_observatory_span`) and `analytics_adapter.rs` (`convert_anomaly` plus
   the methods noted at lines 90 and 434). One commit per adapter.
6. **Restore the disabled tests**, including `test_outlier_type_conversion`
   (`analytics_adapter.rs:434`), gated to the `ecosystem` configuration.
7. **Add the second CI configuration** — `cargo build --locked` and
   `cargo test --locked` for both the default and `--features ecosystem`
   configurations, plus a `cargo build --features benchmarks` check.
8. **Add cross-configuration conformance tests** — for each adapter, one test
   asserting the stub and real implementations agree on a fixed input, so drift
   between them surfaces as a test failure rather than a production surprise.
9. **Delete the stale comments** on lines 43-45 and 167-171 and link this ADR
   from the manifest instead.
10. **Decide the crates.io question** (see "Consequence for publishing"). Record
    the outcome as a follow-up ADR; do not leave `documentation = "docs.rs/..."`
    unexamined.

Steps 3-9 are ordered but only step 3 is blocked on step 1. Steps 5-6 can be
prepared against a local `[patch]` overlay while upstream tags are pending — but
that overlay must not be merged.

## Verification

The decision is implemented when all of the following hold:

1. `grep -c '^# llm-' Cargo.toml` returns `0` — no commented-out ecosystem
   dependency lines remain.
2. `grep -n 'git = ' Cargo.toml` shows every git dependency carrying a `tag = `,
   and no occurrence of `branch = `.
3. `cargo build --locked` succeeds and `cargo tree | grep -c 'LLM-Dev-Ops'`
   returns `0` (default build unchanged).
4. `cargo build --locked --features ecosystem` succeeds, and
   `cargo tree --features ecosystem` shows `llm-sentinel-core`,
   `llm-observatory-core`, `llm-governance-common`, and `llm-analytics-hub`.
5. `cargo tree --features ecosystem --duplicates` reports no duplicate versions
   of any `llm-*` crate.
6. `grep -rn 'feature = "ecosystem"' src/` returns matches in all three of
   `sentinel_adapter.rs`, `observatory_adapter.rs`, and `analytics_adapter.rs`,
   with a `cfg(not(...))` counterpart for each `cfg(...)`.
7. `cargo test --locked` and `cargo test --locked --features ecosystem` both
   pass, and the latter runs `test_outlier_type_conversion`.
8. `cargo build --locked --features benchmarks` succeeds.
9. CI logs show all four configurations executed on a single pull request.
10. Changing an upstream `main` branch does not change this repo's resolved
    dependency graph — verifiable by re-running `cargo update -p <upstream>` and
    observing no lockfile change.

Failure of any single check means the decision is not yet implemented. Check 6
is the one most likely to be skipped, and is the one that distinguishes a real
re-enable from a cosmetic manifest edit.
