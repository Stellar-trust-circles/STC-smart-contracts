# Trust Circle — Smart Contracts

> Soroban smart contracts powering the Stellar Trust Circles platform — a decentralized rotating savings group protocol built on Stellar.

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Built on Stellar](https://img.shields.io/badge/Built%20on-Stellar-7C3AED)](https://stellar.org)
[![Soroban](https://img.shields.io/badge/Soroban-Smart%20Contracts-0F6E56)](https://soroban.stellar.org)
[![Testnet](https://img.shields.io/badge/Deployed-Testnet-854F0B)](https://testnet.stellar.expert/explorer/testnet/contract/CANM5X47IG3AM5JDG6DVGZ24B3RLBNT5653CXRUEUDWF6JERO4YEX6ZS)

---

## What is Stellar Trust Circles?

Rotating savings groups — known as *ajo* in Nigeria, *chama* in Kenya, *tanda* in Latin America — are one of the oldest and most trusted forms of community finance. A group of people contribute a fixed amount regularly, and each cycle one member receives the full pot.

**Stellar Trust Circles** brings this model on-chain:

- Members contribute USDC weekly or monthly
- Payouts rotate automatically via Soroban smart contract
- Every contribution and payout is transparently recorded on-chain
- Members vote on circle rules via on-chain governance
- On-chain reputation builds across multiple circles
- No bank. No middleman. No missed payouts.

---

## Why Stellar?

| Need | Why Stellar fits |
|------|-----------------|
| Cheap transactions | Fees < $0.001 — viable for $5–$50 weekly contributions |
| Fast settlement | 3–5 second finality — payouts land instantly |
| Stablecoin support | Native USDC on Stellar — no volatility risk for savings |
| Smart contracts | Soroban enables trustless rotation logic on-chain |
| Mobile-friendly | Stellar's lightweight protocol suits low-bandwidth users |

---

## How it works

1. A **circle creator** calls `create_circle()` with a member list, contribution amount, and cycle length
2. Each **member** calls `contribute()` before the cycle deadline — USDC is escrowed in the contract
3. At cycle end, `release_payout()` sends the full pot to the next member in rotation
4. Missed contributions are flagged on-chain and reduce the member's reputation score
5. After all members have received once, the circle closes or restarts

---

## Contract functions

| Function | Description |
|----------|-------------|
| `create_circle` | Initialise a new savings circle |
| `contribute` | Deposit USDC for the current cycle |
| `release_payout` | Release the pot to the next member in rotation |
| `get_circle` | Read current circle state (free, no transaction) |
| `get_reputation` | Read a member's on-chain reputation score |
| `has_contributed` | Check if a member contributed in a given cycle |
| `restart_circle` | Restart a completed circle for another rotation |
| `vouch` | Vouch for a new member (requires reputation ≥ 50) |
| `get_vouches` | Return how many vouches an address has received |
| `propose` | Submit a governance proposal to change circle rules |
| `vote` | Vote yes or no on an open proposal |
| `execute_proposal` | Execute a proposal that has reached majority |
| `get_proposal` | Read a proposal by ID |

---

## Reputation system

Every member has an on-chain reputation score:

| Action | Change |
|--------|--------|
| Contribute on time | +10 points |
| Miss a contribution | −20 points (saturates at 0) |

| Score | Status |
|-------|--------|
| 0–49 | New member |
| 50–99 | Building trust |
| 100+ | Trusted |

---

## Architecture

```
┌─────────────────────────────────────────────────┐
│                 Stellar Network                 │
│                                                 │
│  ┌───────────────────────────────────────────┐  │
│  │         Soroban Smart Contract            │  │
│  │  - Circle state & member registry         │  │
│  │  - USDC contribution escrow               │  │
│  │  - Rotation & payout logic                │  │
│  │  - On-chain reputation scores             │  │
│  │  - Social vouching                        │  │
│  │  - Governance proposals & voting          │  │
│  └──────────────────┬────────────────────────┘  │
│                     │ Horizon API / Soroban RPC  │
└─────────────────────┼───────────────────────────┘
                      │
         ┌────────────▼────────────┐
         │   Frontend (React)      │
         │   github.com/Stellar-   │
         │   trust-circles/frontend│
         └─────────────────────────┘
```

---

## Deployed contracts

| Network | Contract ID |
|---------|------------|
| Testnet | [`CANM5X47IG3AM5JDG6DVGZ24B3RLBNT5653CXRUEUDWF6JERO4YEX6ZS`](https://testnet.stellar.expert/explorer/testnet/contract/CANM5X47IG3AM5JDG6DVGZ24B3RLBNT5653CXRUEUDWF6JERO4YEX6ZS) |
| Mainnet | Coming soon |

---

## Repository structure

```
contracts/
├── .github/
│   └── workflows/
│       └── ci.yml               # Build + test on every push
├── contracts/
│   └── trust_circle/
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs            # Full contract logic
│           └── test.rs           # Unit tests
├── Cargo.toml                    # Workspace config
├── CONTRIBUTING.md
├── LICENSE
└── README.md
```

---

## Getting started

### Prerequisites

- Rust v1.84.0 or higher
- Stellar CLI v26.0.0+
- WASM build target

### Install Stellar CLI

```bash
curl -fsSL https://github.com/stellar/stellar-cli/raw/main/install.sh | sh -s -- --install-deps
```

### Add the WASM target

```bash
rustup target add wasm32v1-none
```

### Configure Testnet

```bash
stellar network add \
  --rpc-url https://soroban-testnet.stellar.org:443 \
  --network-passphrase "Test SDF Network ; September 2015" \
  testnet
```

### Generate and fund a keypair

```bash
stellar keys generate alice --network testnet
curl "https://friendbot.stellar.org?addr=$(stellar keys address alice)"
```

### Clone and build

```bash
git clone https://github.com/Stellar-trust-circles/contracts
cd contracts
stellar contract build
```

### Run tests

```bash
cargo test
```

### Deploy to Testnet

```bash
stellar contract deploy \
  --wasm target/wasm32v1-none/release/trust_circle.wasm \
  --source alice \
  --network testnet \
  --alias trust_circle
```

### Invoke a function

```bash
stellar contract invoke \
  --id trust_circle \
  --source alice \
  --network testnet \
  -- \
  get_circle
```

---

## Roadmap

- [x] Core contract: create circle, contribute, rotate payout
- [x] On-chain contribution history and reputation scores
- [x] Testnet deployment
- [x] GitHub Actions CI
- [ ] Social vouching system
- [ ] On-chain governance voting
- [ ] Mainnet deployment
- [ ] Cross-circle reputation aggregation
- [ ] Circle discovery marketplace

---

## Organisation repositories

| Repo | Description |
|------|-------------|
| [contracts](https://github.com/Stellar-trust-circles/contracts) | This repo — Soroban smart contracts |
| [frontend](https://github.com/Stellar-trust-circles/frontend) | React web interface |
| [documents](https://github.com/Stellar-trust-circles/documents) | Integration guide, user guide, architecture |

---

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) to get set up locally and pick up an open issue.

Issues tagged [`good first issue`](../../issues?q=label%3A%22good+first+issue%22) are a great place to start.

---

## License

MIT — see [LICENSE](LICENSE)