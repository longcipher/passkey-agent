# Design: TTL Expiry BDD Scenario Fix

## Summary

The "TTL expiry auto-locks the daemon" BDD scenario currently fakes TTL expiry by manually calling `state.lock()`, which doesn't test the actual time-based auto-locking behavior. This spec fixes the scenario to genuinely test TTL expiry.

## Requirements

- **R1**: The TTL scenario must verify that an unlocked agent transitions to locked after its TTL expires
- **R2**: The test must not rely on `thread::sleep` for the full 15-minute TTL — use a short TTL
- **R3**: The step definition for "the TTL expires" must wait for or simulate actual time passage

## Behavior Traceability Matrix

| Requirement | Scenario | Source File |
|-------------|----------|-------------|
| R1 | TTL expiry auto-locks | `features/daemon_state.feature:23-26` |
| R2 | TTL expiry auto-locks | `bin/pk-daemon/tests/bdd.rs:41-47` |
| R3 | TTL expiry auto-locks | `bin/pk-daemon/tests/bdd.rs:78-85` |

## Approach

The `AgentState` uses `Instant::now()` for TTL tracking, making it hard to mock. Two options:

**Option A (selected)**: Add a `ttl_expiry()` method or make the state accept a configurable TTL. Then use a very short TTL (e.g., 10ms) in the test and sleep briefly.

**Option B**: Add a `#[cfg(test)]` method to inject a custom `Instant` source. More complex, better long-term.

For this spec, **Option A** is sufficient: the step definition for "an unlocked daemon with a short TTL" will create a state with a very short TTL, then "the TTL expires" will sleep past it.

However, `AgentState::UNLOCK_TTL` is a `const`. The simplest fix is to make the TTL configurable (e.g., `AgentState::new_with_ttl(duration)`) while keeping the default at 15 minutes.

## Architecture Decisions

### MADR: Make AgentState TTL configurable

- **Context**: Testing TTL expiry requires a short TTL, but `UNLOCK_TTL` is hardcoded at 15 minutes
- **Decision**: Add `AgentState::new_with_ttl(ttl: Duration)` constructor; keep `AgentState::new()` using the default
- **Consequences**: Minimal API change; backward compatible; test-friendly

## Verification

```bash
cargo test -p pk-daemon --test bdd
# Expected: "TTL expiry auto-locks" scenario passes with actual time-based locking
```
