# Tasks: Graceful Daemon Shutdown

Commit: `c845a6f`

## Task 4.1: Replace signal handler with tokio::select! shutdown

- **Context**: Current `std::process::exit(0)` in spawned task bypasses cleanup
- **Verification**: `cargo check -p pk-daemon` succeeds
- **Loop Type**: TDD
- **Status**: TODO

### Steps

1. **RED**: Verify current behavior — daemon uses `std::process::exit(0)` at `main.rs:79`
2. **GREEN**: Rewrite `bin/pk-daemon/src/main.rs`:
   - Create signal futures for SIGTERM and SIGINT before the server loop
   - Use `tokio::select!` to race `server::start_server(...)` against signal reception
   - On signal: log "Shutting down daemon", remove socket file, return `Ok(())`
   - Remove the spawned signal handler task entirely
   - Remove `std::process::exit(0)` call
3. **REFACTOR**: Run `cargo check -p pk-daemon` — confirm compilation

## Task 4.2: Verify server stops cleanly on signal

- **Context**: The server accept loop must stop when shutdown signal fires
- **Verification**: Manual test or integration test
- **Loop Type**: TDD
- **Status**: TODO

### Steps

1. **RED**: N/A
2. **GREEN**: The `start_server` function runs an infinite `loop { listener.accept() }`. When the `select!` drops the server future, the loop is cancelled. Verify the socket file is removed after shutdown.
3. **REFACTOR**: Add a test that verifies socket file removal (unit test for the cleanup logic)

## Task 4.3: Add unit test for socket cleanup

- **Context**: Verify socket file is removed on shutdown
- **Verification**: `cargo test -p pk-daemon` — new test passes
- **Loop Type**: TDD
- **Status**: TODO

### Steps

1. **RED**: No failing test yet
2. **GREEN**: Add test in `bin/pk-daemon/src/main.rs` (or extract cleanup to a testable function):

   ```rust
   #[test]
   fn socket_cleanup_removes_file() {
       let dir = tempfile::tempdir().unwrap();
       let socket_path = dir.path().join("test.sock");
       std::fs::write(&socket_path, b"").unwrap();
       assert!(socket_path.exists());
       let _ = std::fs::remove_file(&socket_path);
       assert!(!socket_path.exists());
   }
   ```

3. **REFACTOR**: Run `cargo test -p pk-daemon` — confirm test passes
