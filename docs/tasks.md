# passkey-agent Implementation Tasks

## Context

The current workspace is a Rust template with a simple shopping cart example (`crates/common`, `bin/cli-app`). The design document (`docs/design.md`) specifies a complete rewrite into a FIDO2-backed passkey credential agent with a C/S architecture.

### Architecture Overview

- **Workspace**: Cargo workspace with `bin/*` and `crates/*`
- **`pk-core`**: Shared library — crypto primitives, FIDO2 logic, IPC protocol, config serialization
- **`pk-daemon`**: Background daemon — Unix socket listener, state machine (Locked/Unlocked), 15-min TTL with zeroize
- **`pk-cli`**: CLI frontend — `init`, `status`, `lock`, `run` commands, `execvp` process replacement for `sops`

### Cryptography

- DEK/KEK key hierarchy with XChaCha20-Poly1305 encryption
- FIDO2 `hmac-secret` for Passkey-derived KEK
- Argon2id for password-derived KEK
- `secrecy` + `zeroize` for memory-safe secret handling

### Tech Stack (per design + AGENTS.md preferences)

- FIDO2: `authenticator` crate (Mozilla, supports hmac-secret)
- Crypto: `chacha20poly1305`, `argon2`, `rand_core`
- Memory: `secrecy`, `zeroize`
- IPC: `tokio` + `UnixStream`, `interprocess`
- CLI: `clap`, `inquire`
- Daemon: `daemonize` or `ecdysis` (per AGENTS.md)
- Config: `config` crate (TOML)

---

## Tasks

### Task 1: Workspace Restructuring — Replace Template with passkey-agent Layout

**Current state**: Workspace has `bin/cli-app` (shopping cart demo) and `crates/common` (CartItem/Cart/Order types).
**Target state**: Clean workspace with `bin/pk-cli`, `bin/pk-daemon`, and `crates/pk-core`.

**Steps**:

1. Remove `bin/cli-app/` directory entirely
2. Replace `crates/common/` content — remove shopping cart types, replace with `pk-core` crate scaffold
3. Create `crates/pk-core/` with `Cargo.toml` and `src/lib.rs`
4. Create `bin/pk-cli/` with `Cargo.toml` and `src/main.rs`
5. Create `bin/pk-daemon/` with `Cargo.toml` and `src/main.rs`
6. Update root `Cargo.toml` workspace members to `["bin/pk-cli", "bin/pk-daemon", "crates/pk-core"]`
7. Add workspace dependencies: `chacha20poly1305`, `argon2`, `rand_core`, `secrecy`, `zeroize`, `authenticator`, `interprocess`, `inquire`, `daemonize`, `ecdysis`, `age`, `serde_json`, `dirs`, `tokio` (full features), `bincode` or `serde_cbor`
8. Run `just check` to verify clean compilation

**Acceptance criteria**:

- `cargo check --workspace` passes
- No shopping cart code remains
- Three crates exist with correct names and inter-dependencies
- All workspace dependencies declared at root level

---

### Task 2: pk-core — Cryptography Module (`crypto.rs`)

**Scope**: Implement all cryptographic primitives per the DEK/KEK design.

**Steps**:

1. Implement `generate_dek() -> [u8; 32]` using `OsRng`
2. Implement `encrypt_dek(dek: &[u8], kek: &[u8]) -> Vec<u8>` using XChaCha20-Poly1305 with random nonce
3. Implement `decrypt_dek(ciphertext: &[u8], kek: &[u8]) -> Result<[u8; 32], CryptoError>`
4. Implement `derive_kek_password(password: &str, salt: &[u8]) -> Result<[u8; 32], CryptoError>` using Argon2id
5. Implement `encrypt_with_dek(plaintext: &[u8], dek: &[u8]) -> Vec<u8>` using XChaCha20-Poly1305
6. Implement `decrypt_with_dek(ciphertext: &[u8], dek: &[u8]) -> Result<Vec<u8>, CryptoError>`
7. Define `CryptoError` enum with `thiserror`
8. Add unit tests + proptest properties for encrypt/decrypt round-trips
9. Add proptest: `decrypt(encrypt(m, k), k) == m` for arbitrary messages and keys

**Acceptance criteria**:

- All crypto functions compile and pass unit tests
- Proptest round-trip property passes for 1000+ iterations
- `CryptoError` covers all failure modes (invalid key, decryption failure, etc.)
- No `unwrap()` or `expect()` in library code

---

### Task 3: pk-core — FIDO2 Module (`fido.rs`)

**Scope**: Wrap FIDO2 `hmac-secret` extension for Passkey-based KEK derivation.

**Steps**:

1. Implement `generate_passkey_credential(salt: &[u8]) -> Result<(CredentialId, [u8; 32]), FidoError>` — invokes authenticator to create credential with hmac-secret extension
2. Implement `authenticate_passkey(credential_id: &CredentialId, salt: &[u8]) -> Result<[u8; 32], FidoError>` — invokes authenticator assertion with hmac-secret to derive KEK
3. Implement `generate_salt() -> [u8; 32]` and `generate_password_salt() -> [u8; 32]`
4. Define `CredentialId` newtype (wraps `Vec<u8>`)
5. Define `FidoError` enum with `thiserror`
6. Add unit tests (mock where possible; integration tests may require hardware)

**Acceptance criteria**:

- Functions compile with correct signatures
- Salt generation uses `OsRng`
- Error types cover authenticator unavailability, protocol errors
- Module is testable with mocked authenticator for CI

---

### Task 4: pk-core — Config Module (`config.rs`)

**Scope**: Define the persistent configuration format and serialization.

**Config file**: `~/.config/passkey-agent/config.json`

**Data structures**:

```rust
pub struct AgentConfig {
    pub passkey_route: Option<PasskeyRoute>,
    pub password_route: Option<PasswordRoute>,
    pub encrypted_age_public_key: String,
    pub encrypted_age_private_key: Vec<u8>,  // XChaCha20-Poly1305 ciphertext
}

pub struct PasskeyRoute {
    pub credential_id: CredentialId,
    pub salt: [u8; 32],
    pub encrypted_dek: Vec<u8>,  // XChaCha20-Poly1305 ciphertext
}

pub struct PasswordRoute {
    pub salt: [u8; 32],
    pub encrypted_dek: Vec<u8>,  // XChaCha20-Poly1305 ciphertext
}
```

**Steps**:

1. Define all config structs with `serde` derives
2. Implement `AgentConfig::load(path: &Path) -> Result<Self, ConfigError>`
3. Implement `AgentConfig::save(&self, path: &Path) -> Result<(), ConfigError>`
4. Implement `config_path() -> PathBuf` returning `~/.config/passkey-agent/config.json`
5. Define `ConfigError` enum
6. Add unit tests for round-trip serialization

**Acceptance criteria**:

- Config loads/saves correctly as JSON
- Missing config file returns appropriate error
- Invalid JSON returns parse error
- Binary fields (salts, ciphertexts) serialize correctly (base64 or hex)

---

### Task 5: pk-core — IPC Protocol Module (`ipc.rs`)

**Scope**: Define the C/S communication protocol over Unix domain sockets.

**Protocol messages** (using `bincode` or `serde_cbor` for serialization):

```rust
pub enum Request {
    Status,         // Query daemon state
    Unlock { key: Vec<u8> },  // Send decrypted age key to daemon
    Lock,           // Force immediate lock
    GetKey,         // Request age key for exec
}

pub enum Response {
    Status { state: AgentState, ttl_seconds: Option<u64> },
    Ack,
    Locked,
    Key { key: String },
    Error { message: String },
}

pub enum AgentState {
    Locked,
    Unlocked,
}
```

**Steps**:

1. Define `Request` and `Response` enums with `serde` derives
2. Implement `encode_request(req: &Request) -> Vec<u8>` and `decode_request(bytes: &[u8]) -> Result<Request, IpcError>`
3. Implement `encode_response(resp: &Response) -> Vec<u8>` and `decode_response(bytes: &[u8]) -> Result<Response, IpcError>`
4. Implement `AgentClient` struct with methods: `status()`, `unlock()`, `lock()`, `get_key()` — connects to socket, sends request, reads response
5. Define `IpcError` enum
6. Socket path: `~/.cache/passkey-agent/agent.sock`

**Acceptance criteria**:

- Request/response round-trip via bincode/cbor
- Client can connect to a listening socket
- All message variants serialize/deserialize correctly
- Timeout handling for socket operations

---

### Task 6: pk-core — Re-exports and Public API

**Scope**: Clean public API surface for `pk-core` consumers.

**Steps**:

1. Create `pk-core/src/lib.rs` with module declarations: `crypto`, `fido`, `config`, `ipc`
2. Create `pk-core/src/prelude.rs` re-exporting all public types
3. Ensure all public items have doc comments
4. Verify `cargo doc --no-deps -p pk-core` builds cleanly

**Acceptance criteria**:

- `pk-core` compiles as a standalone library
- All modules accessible via prelude
- No public `unwrap()` or `panic!`

---

### Task 7: pk-daemon — State Machine (`state.rs`)

**Scope**: Implement the daemon's in-memory state machine with TTL and zeroize.

**Steps**:

1. Implement `AgentState` struct with:
   - `age_private_key: Option<SecretString>`
   - `unlock_time: Option<Instant>`
   - TTL constant: `const UNLOCK_TTL: Duration = Duration::from_secs(15 * 60)`
2. Implement `unlock(&mut self, key: String)` — stores key, records timestamp
3. Implement `lock(&mut self)` — sets to None, triggers zeroize
4. Implement `is_unlocked(&self) -> bool` — checks TTL expiry
5. Implement `ttl_remaining(&self) -> Option<Duration>`
6. Implement `get_key(&self) -> Option<&str>` — returns key if not expired
7. Add `Drop` impl that calls `zeroize` on the secret
8. Add unit tests for state transitions and TTL expiry

**Acceptance criteria**:

- State transitions: Locked -> Unlocked -> Locked (via TTL or explicit)
- TTL expires correctly after 15 minutes
- `lock()` wipes memory (verified via `SecretString` behavior)
- No key accessible after lock or TTL expiry
- Unit tests pass deterministically

---

### Task 8: pk-daemon — Unix Socket Server (`server.rs`)

**Scope**: Implement the async Unix domain socket listener.

**Steps**:

1. Implement `start_server(socket_path: &Path, state: Arc<tokio::sync::Mutex<AgentState>>) -> Result<(), DaemonError>`
2. Handle incoming connections: read framed message, decode request, dispatch to state, encode response, write back
3. Request dispatch:
   - `Status` → return current state + TTL
   - `Unlock { key }` → call `state.unlock(key)`, return `Ack`
   - `Lock` → call `state.lock()`, return `Locked`
   - `GetKey` → return `Key { key }` if unlocked, else `Error`
4. Use `tokio::net::UnixListener` or `interprocess` for socket management
5. Ensure socket directory exists (`~/.cache/passkey-agent/`)
6. Clean up socket file on graceful shutdown

**Acceptance criteria**:

- Server starts and listens on socket
- All request types handled correctly
- Concurrent connections handled safely via `Arc<Mutex<AgentState>>`
- Socket file cleaned up on shutdown
- Server runs under `tokio` runtime

---

### Task 9: pk-daemon — Main Entry (`main.rs`)

**Scope**: Daemon process entry point with lifecycle management.

**Steps**:

1. Parse CLI args (support `--foreground` flag for debugging)
2. Initialize tracing subscriber
3. Create `AgentState` and wrap in `Arc<tokio::sync::Mutex<>>`
4. Start TTL checker task (periodically checks if key expired, auto-locks)
5. Start Unix socket server
6. Handle SIGTERM/SIGINT for graceful shutdown
7. If not `--foreground`, daemonize (detach from terminal)
8. Use `ecdysis` for graceful restart if applicable

**Acceptance criteria**:

- Daemon starts in background mode by default
- `--foreground` keeps process attached to terminal
- Graceful shutdown on SIGTERM/SIGINT
- Tracing logs to stderr or journald
- Socket created at correct path

---

### Task 10: pk-cli — Command Structure (`commands/`)

**Scope**: Implement all CLI subcommands.

**Subcommands**:

1. `pk init` — Guided initialization flow
2. `pk status` — Query daemon state
3. `pk lock` — Force daemon to lock
4. `pk run <command...>` — Unlock if needed, exec command with `SOPS_AGE_KEY` env

**Steps**:

1. Set up `clap` derive with `#[command(subcommand)]` enum
2. Create `commands/mod.rs` with command dispatch
3. Create `commands/init.rs`
4. Create `commands/status.rs`
5. Create `commands/lock.rs`
6. Create `commands/run.rs`

**Acceptance criteria**:

- `pk --help` shows all subcommands
- Each subcommand has `--help` with description
- `pk` with no args shows help

---

### Task 11: pk-cli — `pk init` Command

**Scope**: Full initialization flow per design document.

**Flow**:

1. Generate `age` keypair
2. Generate DEK via `OsRng`
3. Generate `Salt_1`, invoke FIDO2 authenticator to create passkey credential with `hmac-secret`, get `KEK_1`
4. Encrypt DEK with `KEK_1` → `Encrypted_DEK_Passkey`
5. Prompt user for master password via `inquire`
6. Generate `Password_Salt`, derive `KEK_2` via Argon2id
7. Encrypt DEK with `KEK_2` → `Encrypted_DEK_Password`
8. Encrypt `age` private key with DEK → `Encrypted_Age_Key`
9. Write all metadata to `~/.config/passkey-agent/config.json`

**Steps**:

1. Implement `init_command() -> Result<(), CliError>`
2. Use `inquire` for password prompts with confirmation
3. Display progress indicators via `indicatif`
4. Handle FIDO2 authenticator not available gracefully
5. Validate password strength (minimum length, entropy hints)

**Acceptance criteria**:

- Running `pk init` creates config file
- Config contains all required fields (salts, credential_id, ciphertexts)
- Password prompt requires confirmation match
- Graceful error if FIDO2 device unavailable
- Config file permissions set to `0o600`

---

### Task 12: pk-cli — `pk status` Command

**Scope**: Query and display daemon state.

**Steps**:

1. Connect to daemon socket via `AgentClient`
2. Send `Status` request
3. Display formatted output:
   - `🔓 Unlocked (Expires in 12m 30s)` or
   - `🔒 Locked`
4. Handle daemon not running → `Daemon not running. Run 'pk run' to start it.`

**Acceptance criteria**:

- Correct state displayed
- TTL countdown shown in human-readable format
- Handles daemon unavailable gracefully

---

### Task 13: pk-cli — `pk lock` Command

**Scope**: Send lock command to daemon.

**Steps**:

1. Connect to daemon socket
2. Send `Lock` request
3. Display confirmation: `Agent locked. Memory zeroized.`
4. Handle daemon not running

**Acceptance criteria**:

- Daemon state transitions to Locked
- Confirmation message displayed
- Idempotent (locking an already-locked daemon is OK)

---

### Task 14: pk-cli — `pk run` Command with `execvp`

**Scope**: The core workflow — unlock, inject key, replace process with target command.

**Flow**:

1. Check daemon status via socket
2. If locked:
   a. Read config file
   b. Attempt Passkey unlock first (FIDO2 assertion with salt → KEK_1 → DEK → age key)
   c. If Passkey fails, fall back to password unlock (prompt → Argon2id → KEK_2 → DEK → age key)
   d. Send age key to daemon via `Unlock` request
3. Get age key from daemon via `GetKey` request
4. Use `std::os::unix::process::CommandExt` to `execvp` the target command with `SOPS_AGE_KEY` env

**Steps**:

1. Implement `run_command(args: &[String]) -> Result<!, CliError>` (never returns on success)
2. Implement `ensure_unlocked(client: &AgentClient, config: &AgentConfig) -> Result<(), CliError>`
3. Implement `passkey_unlock(config: &AgentConfig) -> Result<String, CliError>` — returns age key
4. Implement `password_unlock(config: &AgentConfig) -> Result<String, CliError>` — returns age key
5. Implement `exec_command(key: &str, args: &[String]) -> !` — sets env, calls `exec()`

**Acceptance criteria**:

- `pk run sops edit secrets.yaml` works end-to-end
- Passkey unlock triggers OS biometric prompt
- Password fallback works when Passkey unavailable
- `execvp` replaces CLI process (no memory residue)
- `SOPS_AGE_KEY` environment variable set correctly
- Daemon auto-locks after TTL

---

### Task 15: pk-cli — Main Entry and Error Handling

**Scope**: CLI entry point, tracing setup, error reporting.

**Steps**:

1. Set up `clap` with `pk` as command name
2. Initialize `tracing-subscriber` with env filter
3. Wire command dispatch to subcommand handlers
4. Use `eyre` for error reporting in binary
5. Handle daemon-not-running errors with helpful messages
6. `#[allow(clippy::print_stdout, clippy::print_stderr)]` at crate level for CLI

**Acceptance criteria**:

- CLI starts and dispatches correctly
- Errors display with context via `eyre`
- Tracing output visible with `RUST_LOG=debug`

---

### Task 17: Integration and End-to-End Testing

**Scope**: Wire all components together and verify end-to-end.

**Steps**:

1. Test `pk init` → config file created with valid structure
2. Test `pk status` → shows Locked initially
3. Test `pk run` → triggers unlock, execs command
4. Test `pk lock` → daemon locks immediately
5. Test TTL expiry → daemon auto-locks after 15 minutes
6. Test password fallback → works when passkey unavailable
7. Add `just test-all` verification

**Acceptance criteria**:

- Full workflow verified manually or via integration tests
- `just test-all` passes
- `just lint` passes with no warnings

---

### Task 18: Documentation and Polish

**Scope**: Final documentation, README update, and CI verification.

**Steps**:

1. Update `README.md` with passkey-agent description, install instructions, usage examples
2. Ensure all public API items have doc comments
3. Run `just docs` to verify documentation builds
4. Verify `just ci` passes (lint + test + build)
5. Remove any remaining template/shopping-cart references

**Acceptance criteria**:

- README describes passkey-agent project
- `cargo doc` builds without warnings
- `just ci` passes cleanly
- No references to shopping cart or template remain

---

## Task Dependencies

```text
Task 1 (Workspace) 
  ├── Task 2 (Crypto) ──┐
  ├── Task 3 (FIDO2) ───┤
  ├── Task 4 (Config) ──┼── Task 6 (pk-core public API) ──┐
  └── Task 5 (IPC) ─────┘                                │
                                                          ├── Task 7 (Daemon State) ── Task 8 (Daemon Server) ── Task 9 (Daemon Main)
                                                          │
                                                          ├── Task 10 (CLI Commands) ── Task 11 (init)
                                                          │                           ├── Task 12 (status)
                                                          │                           ├── Task 13 (lock)
                                                          │                           └── Task 14 (run) ── Task 15 (CLI Main)
                                                          │
                                                          └── Task 17 (Integration) ── Task 18 (Docs)
```

## Parallelism Opportunities

- Tasks 2-5 (pk-core modules) can be implemented in parallel after Task 1
- Tasks 7-9 (daemon) and Tasks 10-15 (CLI) can proceed in parallel after Task 6
- Tasks 16-18 are sequential and depend on all implementation tasks

## Conditional Work

- **Benchmarking**: N/A — no explicit latency SLA or throughput target in scope
- **Fuzzing**: Conditional — if the IPC protocol parser or crypto wrapper handles untrusted binary input, add `cargo-fuzz` targets to `pk-core`
- **mimalloc/jemallocator**: Conditional — add to `pk-daemon` if memory profiling shows allocation pressure
