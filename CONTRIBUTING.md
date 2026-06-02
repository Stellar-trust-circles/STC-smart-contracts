# Contributing to Trust Circle Contracts

Thanks for your interest in contributing! This repo contains the Soroban smart contracts for the Stellar Trust Circles platform.

---

## Setup

### Prerequisites
- Rust v1.84.0+
- Stellar CLI v26.0.0+
- WASM target: `rustup target add wasm32v1-none`
- A Stellar Testnet keypair

### Clone and build

```bash
git clone https://github.com/Stellar-trust-circles/contracts
cd contracts
stellar contract build
```

### Fund your Testnet account

```bash
curl "https://friendbot.stellar.org?addr=YOUR_PUBLIC_KEY"
```

### Run tests

```bash
cargo test
```

---

## Contribution workflow

1. Browse [open issues](../../issues) — look for `good first issue` or `help wanted`
2. Comment on the issue to claim it before starting
3. Fork the repo and create a branch using the name specified in the issue
4. Make your changes
5. Run `cargo test` and make sure all tests pass
6. Open a pull request using the commit message format in the issue description

---

## Branch naming

| Type | Format | Example |
|------|--------|---------|
| New feature | `feat/description` | `feat/social-vouching` |
| Bug fix | `fix/description` | `fix/payout-overflow` |
| Tests | `test/description` | `test/governance-voting` |
| Docs | `docs/description` | `docs/contract-functions` |

---

## Commit message format

Follow this pattern:
```
type: short description of what changed
```

Types: `feat`, `fix`, `test`, `docs`, `chore`, `refactor`

---

## Code style

- All functions must have a doc comment explaining what they do
- Every new function needs at least one unit test
- Keep assertions clear — error messages should explain what went wrong
- No unused imports or dead code

---

## Need help?

Open a [GitHub Discussion](../../discussions) or drop a message in the issue thread.