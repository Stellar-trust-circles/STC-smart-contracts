# Stellar Trust Circles

> Decentralized rotating savings groups — powered by Stellar and Soroban smart contracts.

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Built on Stellar](https://img.shields.io/badge/Built%20on-Stellar-7C3AED)](https://stellar.org)
[![Testnet Contract](https://img.shields.io/badge/Soroban-Testnet-0F6E56)](https://testnet.stellar.expert)

---

## What is this?

Rotating savings groups — known as *ajo* in Nigeria, *chama* in Kenya, *tanda* in Latin America — are one of the oldest and most trusted forms of community finance. A group of people contribute a fixed amount regularly, and each cycle one member receives the full pot.

**Stellar Trust Circles** brings this model on-chain:

- Members contribute stablecoins (USDC) weekly or monthly
- Payouts rotate automatically via Soroban smart contract
- Every contribution and payout is transparently recorded on-chain
- Members vote on circle rules (contribution amount, frequency, size)
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

```
Members join circle → contribute USDC each cycle → Soroban contract
rotates payout to next member → on-chain history builds reputation
```

1. A **circle creator** deploys the contract with: member list, contribution amount, cycle length (weekly/monthly), payout order
2. Each **member** calls `contribute()` before the cycle deadline
3. At cycle end, the contract calls `release_payout()` — sending the full pot to the next member in rotation
4. Missed contributions are flagged on-chain and affect the member's reputation score
5. After all members have received once, the circle either **closes** or **restarts**

---

## Architecture

```
┌─────────────────────────────────────────────┐
│              Stellar Network                │
│                                             │
│  ┌─────────────────────────────────────┐   │
│  │       Soroban Smart Contract        │   │
│  │  - Circle state & member registry   │   │
│  │  - Contribution escrow (USDC)       │   │
│  │  - Rotation & payout logic          │   │
│  │  - On-chain reputation scores       │   │
│  └──────────────┬──────────────────────┘   │
│                 │ Horizon API               │
└─────────────────┼───────────────────────────┘
                  │
     ┌────────────▼────────────┐
     │   Node.js SDK Layer     │
     │  stellar-trust-circles  │
     └────────────┬────────────┘
                  │
        ┌─────────▼──────────┐
        │   CLI / Mobile UI  │
        └────────────────────┘
```

---

## Testnet Deployment

| Item | Value |
|------|-------|
| Network | Stellar Testnet |
| Contract ID | `CANM5X47IG3AM5JDG6DVGZ24B3RLBNT5653CXRUEUDWF6JERO4YEX6ZS` *(update after deploy)* |
| USDC Asset | `USDC:GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5` (Testnet) |

---

## Project Structure

```
stellar-trust-circles/
├── contracts/
│   └── trust_circle/
│       ├── src/
│       │   └── lib.rs          # Soroban smart contract
│       └── Cargo.toml
├── src/
│   ├── stellar.js              # Stellar SDK integration
│   ├── circle.js               # Circle management logic
│   └── reputation.js           # On-chain reputation helpers
├── cli.js                      # CLI demo tool
├── .env.example                # Environment config template
├── package.json
├── FUNDING.json
├── CONTRIBUTING.md
└── README.md
```

---

## Getting Started

### Prerequisites

- Node.js v18+
- Rust + Soroban CLI (`cargo install --locked soroban-cli`)
- A Stellar Testnet keypair ([generate one here](https://laboratory.stellar.org/#account-creator))

### Install

```bash
git clone https://github.com/Marvell69/stellar-trust-circles
cd stellar-trust-circles
npm install
cp .env.example .env
# Fill in your STELLAR_SECRET_KEY in .env
```

### Build & deploy the contract

```bash
cd contracts/trust_circle
soroban contract build
soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/trust_circle.wasm \
  --network testnet \
  --source YOUR_SECRET_KEY
```

### Run the CLI demo

```bash
# Create a new circle
node cli.js create --name "Lagos Squad" --amount 10 --members 5 --cycle weekly

# Contribute to a circle
node cli.js contribute --circle <CONTRACT_ID> --amount 10

# Check circle status
node cli.js status --circle <CONTRACT_ID>
```

---

## Roadmap

- [x] Core contract: create circle, contribute, rotate payout
- [x] On-chain contribution history
- [x] Testnet deployment
- [ ] Social verification (vouching system)
- [ ] Governance voting on circle rules
- [ ] Mobile-first UI (React Native)
- [ ] Cross-circle reputation score
- [ ] Circle discovery marketplace

---

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for how to run the project locally and pick up an open issue.

Issues tagged [`good first issue`](../../issues?q=label%3A%22good+first+issue%22) are a great place to start

---

## License

MIT — see [LICENSE](LICENSE)

---

## Built with

- [Stellar](https://stellar.org) — Layer 1 blockchain
- [Soroban](https://soroban.stellar.org) — Stellar smart contracts
- [Stellar JS SDK](https://github.com/stellar/js-stellar-sdk)
- [Horizon API](https://developers.stellar.org/api/horizon)