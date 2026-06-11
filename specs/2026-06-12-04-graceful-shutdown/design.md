# Design: Graceful Daemon Shutdown

## Summary

The daemon's signal handler calls `std::process::exit(0)` inside a spawned tokio task, which bypasses Drop implementations, skips tokio shutdown, and may leave the socket file on disk. This spec implements proper graceful shutdown.

## Requirements

- **R1**: On SIGTERM/SIGINT, the daemon must remove the socket file before exiting
- **R2**: The daemon must wait for in-flight request handlers to complete (or timeout)
- **R3**: The shutdown must not use `std::process::exit()` — rely on normal return from `main()`
- **R4**: The daemon must log shutdown progress

## Behavior Traceability Matrix

| Requirement | Scenario | Source File |
|-------------|----------|-------------|
| R1 | Socket cleanup on shutdown | `bin/pk-daemon/src/main.rs` |
| R2 | Drain in-flight requests | `bin/pk-daemon/src/server.rs` |
| R3 | No forced exit | `bin/pk-daemon/src/main.rs:79` |
| R4 | Shutdown logging | `bin/pk-daemon/src/main.rs` |

## Approach

Replace the spawned signal handler task with a `tokio::signal::ctrl_c()` or `tokio::signal::unix::signal()` future awaited in `main()`. Use a `tokio::sync::broadcast` channel or `CancellationToken` to通知 the server loop to stop accepting new connections.

**Selected approach**: Use `tokio_util::sync::CancellationToken` (add as dependency) or a simple `tokio::sync::watch` channel to signal shutdown. The server loop checks the flag and breaks.

Since adding `tokio_util` is a new dependency, use a simpler approach: `tokio::select!` in `main()` between the server future and the signal future. When signal fires, drop the listener (which stops accept), remove socket, return.

## Architecture Decisions

### MADR: Use tokio::select! for shutdown coordination

- **Context**: Current signal handling uses `std::process::exit(0)` which bypasses cleanup
- **Decision**: Use `tokio::select!` in main to race server against signal; on signal, clean up and return
- **Consequences**: No new dependencies; clean shutdown; socket file always removed; Drop impls run

## Verification

```bash
cargo test -p pk-daemon
# Expected: all tests pass
# Manual: start daemon, send SIGTERM, verify socket file removed and process exits cleanly
```
