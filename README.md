# passkey-agent

A next-generation, passwordless-first local credential agent and SOPS runtime environment.

**Core principles**: biometric unlock, zero private key persistence, automatic memory zeroization, side-channel attack resistance.

## Architecture

Client-server model communicating over Unix domain sockets:

- **`pk` (CLI)** — handles user interaction, FIDO2 passkey prompts (Touch ID / Windows Hello), collects passwords, initiates RPC requests, and replaces itself with `sops` via `execvp`
- **`pk-daemon`** — background daemon maintaining `Locked`/`Unlocked` state with 15-minute TTL and `zeroize` memory wiping
- **`pk-core`** — shared library providing cryptographic primitives, FIDO2 logic, IPC protocol, and configuration management

## Cryptography

- **DEK/KEK hierarchy**: a random 32-byte Data Encryption Key encrypts the `age` private key; two Key Encryption Keys (Passkey-derived via FIDO2 `hmac-secret`, password-derived via Argon2id) each encrypt the DEK
- **XChaCha20-Poly1305** for all symmetric encryption (side-channel resistant without AES-NI)
- **`secrecy` + `zeroize`** for memory-safe secret handling

## Quick Start

```bash
# Build the workspace
just build

# Initialize the agent
pk init

# Check daemon status
pk status

# Run sops with automatic unlock
pk run sops edit secrets.yaml

# Lock immediately
pk lock
```

## Testing

```bash
just test       # Unit + property tests
just test-all   # Both suites
```

## Project Structure

```text
passkey-agent/
├── bin/
│   ├── pk-cli/          # CLI frontend (binary: pk)
│   └── pk-daemon/       # Background daemon (binary: pk-daemon)
├── crates/
│   └── pk-core/         # Shared library
└── docs/
    ├── design.md        # Architecture design document
    └── tasks.md         # Implementation task breakdown
```

## License

Apache-2.0
