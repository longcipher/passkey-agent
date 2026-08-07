# Tasks: FIDO2 Passkey Integration

Commit: `c845a6f`

## Task 5.1: Enable authenticator crate dependency

- **Context**: The `authenticator` crate is commented out in pk-core Cargo.toml
- **Verification**: `cargo check -p pk-core` succeeds
- **Loop Type**: TDD
- **Status**: TODO

### Steps

1. **RED**: Verify current state — `authenticator` is commented out at `crates/pk-core/Cargo.toml:26`
2. **GREEN**: Uncomment `authenticator = { workspace = true }` in `crates/pk-core/Cargo.toml`
3. **REFACTOR**: Run `cargo check -p pk-core` — confirm compilation (may fail if API changed; see Task 5.2)

## Task 5.2: Implement generate_passkey_credential

- **Context**: Currently returns `Err(AuthenticatorUnavailable)` — must invoke platform authenticator
- **Verification**: `cargo check -p pk-core` succeeds with real implementation
- **Loop Type**: TDD
- **Status**: TODO

### Steps

1. **RED**: Run `cargo test -p pk-core` — existing tests pass (stub returns Err)
2. **GREEN**: Edit `crates/pk-core/src/fido.rs`:
   - Import `authenticator::{Authenticator, Registration, Directory}`
   - In `generate_passkey_credential`:
     - Create an `Authenticator` instance
     - Call `authenticator.register(...)` with the salt
     - Map result to `(CredentialId, [u8; 32])`
     - Handle errors: `UserCancelled`, `AuthenticatorUnavailable`, `ProtocolError`
   - Reference the `authenticator` crate docs for exact API
3. **REFACTOR**: Run `cargo check -p pk-core` — confirm compilation

## Task 5.3: Implement authenticate_passkey

- **Context**: Currently returns `Err(AuthenticatorUnavailable)` — must invoke platform authenticator
- **Verification**: `cargo check -p pk-core` succeeds
- **Loop Type**: TDD
- **Status**: TODO

### Steps

1. **RED**: Run `cargo test -p pk-core` — existing tests pass
2. **GREEN**: Edit `crates/pk-core/src/fido.rs`:
   - In `authenticate_passkey`:
     - Create an `Authenticator` instance
     - Call `authenticator.authenticate(...)` with credential ID and salt
     - Map result to `[u8; 32]` KEK
     - Handle errors appropriately
3. **REFACTOR**: Run `cargo check -p pk-core` — confirm compilation

## Task 5.4: Update init command for real passkey flow

- **Context**: `pk init` currently handles passkey gracefully if unavailable; must work with real implementation
- **Verification**: `cargo check -p pk-cli` succeeds
- **Loop Type**: TDD
- **Status**: TODO

### Steps

1. **RED**: Run `cargo check -p pk-cli` — should compile
2. **GREEN**: Review `bin/pk-cli/src/commands/init.rs` — the existing code already handles `Ok((credential_id, kek))` from `generate_passkey_credential`. No changes needed if the return type matches.
3. **REFACTOR**: Run `cargo check -p pk-cli` — confirm compilation

## Task 5.5: Update key.rs for real passkey auth

- **Context**: `try_unlock_passkey` calls `authenticate_passkey` — must work with real implementation
- **Verification**: `cargo check -p pk-cli` succeeds
- **Loop Type**: TDD
- **Status**: TODO

### Steps

1. **RED**: Run `cargo check -p pk-cli` — should compile
2. **GREEN**: Review `bin/pk-cli/src/key.rs` — the existing code already handles the `Result<[u8; 32], FidoError>` return. No changes needed if the return type matches.
3. **REFACTOR**: Run `cargo check -p pk-cli` — confirm compilation

## Task 5.6: Add mock-based unit tests for FIDO2

- **Context**: Real FIDO2 requires hardware; need mock tests for CI
- **Verification**: `cargo test -p pk-core` — new tests pass
- **Loop Type**: TDD
- **Status**: TODO

### Steps

1. **RED**: No failing test yet
2. **GREEN**: Add tests in `crates/pk-core/src/fido.rs`:

   ```rust
   #[test]
   fn generate_salt_produces_32_bytes() { ... } // already exists

   #[test]
   fn credential_id_wraps_bytes() { ... } // already exists

   // New: test error handling when authenticator unavailable
   #[test]
   fn generate_passkey_returns_error_when_no_authenticator() {
       let result = generate_passkey_credential(&[0u8; 32]);
       assert!(result.is_err());
   }
   ```

3. **REFACTOR**: Run `cargo test -p pk-core` — confirm tests pass
