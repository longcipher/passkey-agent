# Design: BDD Harness Fix for pk-core

## Summary

The pk-core BDD test harness (`crates/pk-core/tests/bdd.rs`) runs all feature files but defines no step implementations, causing every scenario to be skipped. This spec fixes the harness to either reuse pk-daemon's step definitions or define its own, ensuring BDD scenarios actually execute.

## Requirements

- **R1**: All Gherkin scenarios in `features/*.feature` must execute (not be skipped) when running `cargo test -p pk-core --test bdd`
- **R2**: Step definitions must be thin — delegate to domain crates, no business logic in test code
- **R3**: The pk-daemon BDD harness must continue to pass after changes

## Behavior Traceability Matrix

| Requirement | Scenario | Source File |
|-------------|----------|-------------|
| R1 | All 4 daemon_state scenarios | `features/daemon_state.feature` |
| R2 | All step definitions | `crates/pk-core/tests/bdd.rs` |
| R3 | pk-daemon BDD pass | `bin/pk-daemon/tests/bdd.rs` |

## Approach

The current pk-core `bdd.rs` creates an empty `PkCoreWorld` with no step definitions. The fix is to:

1. Define a `PkCoreWorld` that contains the same state as `DaemonWorld` (it already wraps `AgentState`)
2. Add all step definitions that delegate to `pk_daemon::state::AgentState`
3. Since pk-core is a library crate and the step definitions need `AgentState` from pk-daemon, we need pk-daemon as a dev-dependency of pk-core

**Decision**: Make pk-daemon a dev-dependency of pk-core so pk-core BDD can reuse the same domain logic. This is clean because pk-core is the shared library and pk-daemon depends on pk-core (not circular).

## Architecture Decisions

### MADR: Reuse pk-daemon's AgentState via dev-dependency

- **Context**: pk-core BDD needs to test daemon state management, but AgentState lives in pk-daemon
- **Decision**: Add `pk-daemon` as a dev-dependency of pk-core for BDD tests only
- **Consequences**: Clean separation; BDD tests in pk-core exercise the full stack through the library boundary; no code duplication

## BDD/TDD Strategy

- **Framework**: cucumber-rs (already in use)
- **Runner**: `cargo test -p pk-core --test bdd`
- **Pattern**: RED → GREEN → REFACTOR per step definition

## Code Simplification Constraints

- Step definitions must be one-liners delegating to AgentState methods
- No business logic in test code
- Match existing code style in `bin/pk-daemon/tests/bdd.rs`

## Verification

```bash
cargo test -p pk-core --test bdd
# Expected: 4 scenarios, 14 steps, all pass
```
