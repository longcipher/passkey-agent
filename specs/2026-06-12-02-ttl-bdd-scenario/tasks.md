# Tasks: TTL Expiry BDD Scenario Fix

Commit: `c845a6f`

## Task 2.1: Make AgentState TTL configurable

- **Context**: `UNLOCK_TTL` is a const; tests need a short TTL to verify expiry
- **Verification**: `cargo test -p pk-daemon` — all existing tests pass
- **Loop Type**: TDD
- **Status**: TODO

### Steps

1. **RED**: Run `cargo test -p pk-daemon` — capture current pass count
2. **GREEN**: Edit `bin/pk-daemon/src/state.rs`:
   - Add a `ttl: Duration` field to `AgentState`
   - Add `AgentState::new_with_ttl(ttl: Duration)` constructor
   - Change `AgentState::new()` to call `Self::new_with_ttl(UNLOCK_TTL)`
   - Update `is_unlocked()` and `ttl_remaining()` to use `self.ttl` instead of `UNLOCK_TTL`
3. **REFACTOR**: Run `cargo test -p pk-daemon` — confirm all existing tests still pass

## Task 2.2: Fix the TTL BDD scenario step definitions

- **Context**: The "TTL expiry" scenario needs to actually wait for time to pass
- **Verification**: `cargo test -p pk-daemon --test bdd` — "TTL expiry" scenario passes with real time-based locking
- **Loop Type**: BDD+TDD
- **Status**: TODO

### Steps

1. **RED**: Run `cargo test -p pk-daemon --test bdd` — capture current output (all 4 pass, but TTL test is fake)
2. **GREEN**: Edit `bin/pk-daemon/tests/bdd.rs`:
   - Modify `unlocked_daemon_short_ttl` step to use `AgentState::new_with_ttl(Duration::from_millis(50))`
   - Modify `ttl_expires` step to `tokio::time::sleep(Duration::from_millis(100)).await` then check `is_unlocked()`
3. **REFACTOR**: Run `cargo test -p pk-daemon --test bdd` — confirm "TTL expiry" scenario actually tests time-based locking

## Task 2.3: Add unit test for configurable TTL

- **Context**: Verify the new `new_with_ttl` constructor works correctly
- **Verification**: `cargo test -p pk-daemon` — new test passes
- **Loop Type**: TDD
- **Status**: TODO

### Steps

1. **RED**: No failing test yet
2. **GREEN**: Add test in `bin/pk-daemon/src/state.rs`:

   ```rust
   #[test]
   fn custom_ttl_locks_after_expiry() {
       let mut state = AgentState::new_with_ttl(Duration::from_millis(10));
       state.unlock("key".to_string());
       std::thread::sleep(Duration::from_millis(20));
       assert!(!state.is_unlocked());
   }
   ```

3. **REFACTOR**: Run `cargo test -p pk-daemon` — confirm new test passes
