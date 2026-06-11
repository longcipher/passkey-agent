# Tasks: IPC Socket Authentication

Commit: `c845a6f`

## Task 3.1: Add peer credential check to server accept loop

- **Context**: Server must verify connecting process UID matches daemon UID
- **Verification**: `cargo check -p pk-daemon` succeeds
- **Loop Type**: TDD
- **Status**: TODO

### Steps

1. **RED**: Verify current behavior — server accepts any connection
2. **GREEN**: Edit `bin/pk-daemon/src/server.rs`:
   - After `listener.accept()`, call `stream.peer_cred()` to get connecting UID
   - Get daemon UID via `std::fs::metadata(socket_path)?.uid()` (Unix-only)
   - If UIDs don't match, log warning and close connection without processing
   - Add `#[cfg(unix)]` guards since `peer_cred()` is Unix-only
3. **REFACTOR**: Run `cargo check -p pk-daemon` — confirm compilation

## Task 3.2: Add BDD scenario for auth rejection

- **Context**: Verify unauthorized connections are rejected
- **Verification**: `cargo test -p pk-daemon --test bdd` — new scenario passes
- **Loop Type**: BDD+TDD
- **Status**: TODO

### Steps

1. **RED**: Add new Gherkin scenario to `features/daemon_state.feature`:

   ```gherkin
   Scenario: Unauthorized connection is rejected
     Given a running daemon
     When a different user connects to the socket
     Then the connection should be rejected
   ```

2. **GREEN**: Add step definitions to `bin/pk-daemon/tests/bdd.rs`:
   - Since testing cross-user is complex, use a unit test approach instead
   - Add a unit test in `server.rs` that verifies peer credential comparison logic
3. **REFACTOR**: Run `cargo test -p pk-daemon` — confirm all tests pass

## Task 3.3: Add unit test for credential comparison

- **Context**: Verify the UID comparison logic works correctly
- **Verification**: `cargo test -p pk-daemon` — new test passes
- **Loop Type**: TDD
- **Status**: TODO

### Steps

1. **RED**: No failing test yet
2. **GREEN**: Add test in `bin/pk-daemon/src/server.rs` (or a new test module):

   ```rust
   #[test]
   fn same_uid_matches() {
       // Test that daemon UID matches itself
   }

   #[test]
   fn different_uid_rejects() {
       // Test that a different UID is rejected
   }
   ```

3. **REFACTOR**: Run `cargo test -p pk-daemon` — confirm tests pass
