# Tasks: BDD Harness Fix for pk-core

Commit: `c845a6f`

## Task 1.1: Add pk-daemon as dev-dependency of pk-core

- **Context**: pk-core needs access to `AgentState` from pk-daemon for BDD step definitions
- **Verification**: `cargo check -p pk-core --tests` succeeds
- **Loop Type**: TDD
- **Status**: 🟢 DONE

### Steps

1. **RED**: Verify current state — `cargo test -p pk-core --test bdd` shows 4 skipped scenarios ✅
2. **GREEN**: Edit `crates/pk-core/Cargo.toml` to add under `[dev-dependencies]`:

   ```toml
   pk-daemon = { path = "../../bin/pk-daemon" }
   ``` ✅
3. **REFACTOR**: Run `cargo check -p pk-core --tests` to verify compilation ✅

## Task 1.2: Implement PkCoreWorld with step definitions

- **Context**: The empty `PkCoreWorld` needs state fields and all step implementations
- **Verification**: `cargo test -p pk-core --test bdd` shows 4 scenarios, 14 steps, all pass
- **Loop Type**: BDD+TDD
- **Status**: 🟢 DONE

### Steps

1. **RED**: Run `cargo test -p pk-core --test bdd` — capture "4 scenarios (4 skipped)" output ✅
2. **GREEN**: Rewrite `crates/pk-core/tests/bdd.rs` with:
   - `PkCoreWorld` struct with `state: Option<Arc<Mutex<AgentState>>>`, `was_unlocked: bool`, `key_retrievable: bool`
   - All 8 step definitions (`given`, `when`, `then`) delegating to `AgentState` methods
   - Copy the pattern from `bin/pk-daemon/tests/bdd.rs` but using `pk_daemon::state::AgentState` ✅
3. **REFACTOR**: Run `cargo test -p pk-core --test bdd` — capture "4 scenarios (4 passed), 14 steps (14 passed)" output ✅

## Task 1.3: Verify pk-daemon BDD still passes

- **Context**: Changes to pk-core must not break pk-daemon's existing BDD
- **Verification**: `cargo test -p pk-daemon --test bdd` shows 4 scenarios, 14 steps, all pass
- **Loop Type**: TDD
- **Status**: 🟢 DONE

### Steps

1. **RED**: N/A (verification only)
2. **GREEN**: Run `cargo test -p pk-daemon --test bdd` — confirm all pass ✅
3. **REFACTOR**: N/A
