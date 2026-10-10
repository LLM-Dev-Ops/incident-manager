# Node workspace recovery evidence

Date: 2026-10-10
Baseline: `bcb8c8d027997c3363a9fd0ebd0ddc9c086227c6`

## Frozen acceptance contract

The candidate must install from the lockfile; build and test every configured npm workspace; report zero production audit findings; expose a usable no-op telemetry implementation; preserve valid registration and explicit configuration overrides; retain RuVector; and avoid persistence changes. Rust validation remains independent and cannot be counted as passing when its private dependency cannot be resolved.

## Measured result

| Check | Baseline | Candidate |
| --- | ---: | ---: |
| `npm ci --ignore-scripts` | pass | pass |
| `npm run build --workspaces --if-present` | fail (6 TypeScript errors) | pass |
| `npm run test --workspaces --if-present` | Node 24 pass; Node 20 fails on an unmatched functions glob | pass; 106 tests including 2 new contract tests |
| `npm audit --omit=dev --json` | 38 findings (2 critical, 26 high, 10 moderate) | 0 findings |
| Production dependency count | 435 | 12 |
| Escalation CLI help | fail because workspace does not build | pass |

The security reduction comes from removing the unused root `claude-flow` production dependency and regenerating the lockfile. Repository search found no source, script, or workflow using that package.

The functions workspace had no tests but invoked `node --test dist/**/*.test.js`. Node 20 treats the unmatched glob as an error while Node 24 returned an empty successful suite locally. Using the test runner's native discovery (`node --test`) is portable across the declared Node 20 floor and will automatically discover tests when they are added.

## MetaHarness retention record

- MetaHarness `0.4.17` scored the candidate at 71/100 harness fit, with compile 100, task coverage 100, tool safety 95, memory 49, and all six detected constraints satisfied.
- Darwin `0.10.3` ran one deterministic generation with two children, seed `2026101008`, real sandbox, fixed evaluator/model contract. Baseline and both children scored `0.435`; no harness mutation was retained.
- Flywheel `0.1.12` ran a live six-item holdout and frozen two-item architecture anchor. The verified project candidate moved from `2/6` to `6/6` (`0.333333` to `1.0`), held the anchor at `1.0`, and was promoted. Gate fingerprint: `c5942e7c9bf8f28fe7af47a6ec3887d11eeb3f8c8e8601cc6a7a54969ea0b981`. Ed25519 replay verification passed with chain `gen1(sourceVariant) -> gen0(root)`.

## Known independent blocker

`cargo +1.84.1 check --workspace` cannot resolve `llm-sentinel-core`: that tag refers to the private `LLM-Dev-Ops/infra` repository and the isolated shell runtime receives HTTP 401. This is recorded as blocked, not as Rust validation success. The source candidate does not alter Rust or persistence code.

## Rollback

Revert the candidate commit. That restores the original dependency graph and TypeScript sources atomically; no data migration or persistence rollback is involved.
