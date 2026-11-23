# passkey-agent

A next-generation, passwordless-first local credential agent and SOPS runtime environment.

**Core principles**: biometric unlock, zero private key persistence, automatic memory zeroization, side-channel attack resistance.

## Pre-commit Hooks

This repository ships a [prek](https://github.com/j178/prek) configuration in
`.pre-commit-config.yaml` for fast, commit-sized checks. It runs:

- file hygiene: trailing whitespace, final newline, merge conflicts, large
  files, YAML/TOML/JSON syntax, broken symlinks, and private keys
- [Kingfisher](https://github.com/mongodb/kingfisher) secret scanning on the
  staged files, with `--no-validate --redact` so the hook stays offline and
  never prints a detected secret
- `typos`, `rumdl`, `cargo fmt --check`, and `cargo clippy -D warnings`

Tests are deliberately excluded: `cargo test` stays in `just test` and CI.
The first Kingfisher run downloads and caches the pinned binary; later runs
reuse it. The auto-download hook needs `bash`, `curl`, and `tar` (`unzip` on
Windows); on Windows use the Bash shipped with Git for Windows.

The hooks call `typos`, `rumdl`, and the project toolchain directly, so run
`just setup` once first. Then install the Git hook once per clone (install
prek with `brew install prek`, `uv tool install prek`, or
`cargo install --locked prek`):

```bash
prek install
```

Run every hook on every tracked file:

```bash
just hooks
```

`repo: builtin` is a prek extension, so this configuration is meant to be
run with prek rather than upstream pre-commit.

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
