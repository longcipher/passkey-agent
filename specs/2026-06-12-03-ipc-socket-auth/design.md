# Design: IPC Socket Authentication

## Summary

The Unix domain socket server accepts connections from any local process without authentication. A malicious process could connect to the socket and send `GetKey` to retrieve the age private key. This spec adds peer credential verification using `SO_PEERCRED` to restrict access to the owning user.

## Requirements

- **R1**: The daemon must verify that connecting processes belong to the same Unix user as the daemon
- **R2**: Unauthorized connections must receive an error response and be disconnected
- **R3**: The authentication check must have negligible performance overhead
- **R4**: The feature must work on Linux and macOS (the two target platforms)

## Behavior Traceability Matrix

| Requirement | Scenario | Source File |
|-------------|----------|-------------|
| R1 | Same-user connection succeeds | `features/daemon_state.feature` (new scenarios) |
| R2 | Different-user connection rejected | `features/daemon_state.feature` (new scenarios) |
| R3 | Server accept loop performance | `bin/pk-daemon/src/server.rs` |
| R4 | Cross-platform support | `bin/pk-daemon/src/server.rs` |

## Approach

Use `tokio::net::UnixStream::peer_cred()` (available via `tokio::net::UCred`) to get the connecting process's UID/GID. Compare against the daemon's own UID (`std::os::unix::fs::MetadataExt::uid()` on the socket file, or `nix::unistd::getuid()`).

Since adding `nix` just for `getuid()` is heavy, use `std::os::unix::process::CommandExt` or simply read `/proc/self/status` on Linux. However, the simplest portable approach is `users::get_current_uid()` from the `users` crate, or just compare against `unsafe { libc::getuid() }` — but we want to avoid `unsafe`.

**Selected approach**: Use `std::os::unix::fs::MetadataExt` on the socket file to get the daemon's UID, then compare with `peer_cred()` from tokio.

## Architecture Decisions

### MADR: Use tokio peer credentials for auth

- **Context**: Unix socket has no built-in auth; any local process can connect
- **Decision**: Verify connecting process UID matches daemon UID via `SO_PEERCRED`
- **Consequences**: Only the owning user can communicate with the daemon; no auth overhead beyond a single syscall; works on Linux and macOS

## Verification

```bash
cargo test -p pk-daemon
# Expected: all tests pass, including new auth-related tests
```
