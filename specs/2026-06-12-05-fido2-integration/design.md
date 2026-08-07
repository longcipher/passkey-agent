# Design: FIDO2 Passkey Integration

## Summary

The core feature of passkey-agent — biometric passkey unlock — is stubbed. `generate_passkey_credential` and `authenticate_passkey` in `fido.rs` always return `Err(AuthenticatorUnavailable)`. This spec designs the integration with the `authenticator` crate to enable real FIDO2 CTAP2 hmac-secret interaction.

## Requirements

- **R1**: `generate_passkey_credential` must invoke the platform authenticator to create a credential with hmac-secret extension
- **R2**: `authenticate_passkey` must invoke the platform authenticator to authenticate and derive a KEK via hmac-secret
- **R3**: The implementation must work on macOS (Touch ID) and Linux (u2f-hid/fido2)
- **R4**: The `authenticator` crate dependency must be enabled (currently commented out in pk-core Cargo.toml)
- **R5**: Operations must handle user cancellation gracefully (return `FidoError::UserCancelled`)

## Behavior Traceability Matrix

| Requirement | Scenario | Source File |
|-------------|----------|-------------|
| R1 | Passkey credential generation | `crates/pk-core/src/fido.rs:42-49` |
| R2 | Passkey authentication | `crates/pk-core/src/fido.rs:54-60` |
| R3 | Cross-platform support | `crates/pk-core/Cargo.toml:26` |
| R4 | Enable authenticator dep | `crates/pk-core/Cargo.toml:26` |
| R5 | User cancellation handling | `crates/pk-core/src/fido.rs:25-26` |

## Approach

The `authenticator` crate (Mozilla) provides CTAP2 protocol support with hmac-secret extension. The flow:

1. **Registration** (`generate_passkey_credential`):
   - Create a `Registration` with the salt
   - Call `authenticator.register()` to trigger platform prompt
   - Return credential ID and hmac-secret output as KEK

2. **Authentication** (`authenticate_passkey`):
   - Create an `Assertion` with credential ID and salt
   - Call `authenticator.authenticate()` to trigger platform prompt
   - Return hmac-secret output as KEK

**Key challenge**: The `authenticator` crate API has changed across versions. Need to verify the exact API for version 0.5.0 (already in workspace deps).

**Platform considerations**:

- macOS: Uses Touch ID via the platform authenticator
- Linux: Requires u2f-hid or fido2 service running
- Windows: Not a current target

## Architecture Decisions

### MADR: Use authenticator crate (Mozilla) for FIDO2

- **Context**: Need CTAP2 hmac-secret for KEK derivation
- **Decision**: Use `authenticator` crate v0.5.0 already in workspace dependencies
- **Consequences**: Cross-platform FIDO2 support; NSS build dependency on Linux; requires platform authenticator daemon on Linux

## Test Strategy

- **Unit Tests**: Mock-based tests for credential creation/auth flow
- **Integration Tests**: Manual testing on macOS Touch ID (FIDO2 requires hardware)

## Verification

```bash
cargo check -p pk-core
# Expected: compiles with authenticator crate enabled
cargo test -p pk-core
# Expected: existing tests pass, new mock-based tests pass
# Manual: pk init on macOS triggers Touch ID prompt
```
