# Contributing to Stellar Trust Circles

Thanks for your interest! This project is open source and welcomes contributions of all kinds — bug fixes, new features, documentation improvements, and ideas.

---

## Quick start

### Prerequisites
- Node.js 18+
- Rust + Soroban CLI
- A Stellar Testnet account ([create one here](https://laboratory.stellar.org/#account-creator))

### Setup

```bash
git clone https://github.com/YOUR_USERNAME/stellar-trust-circles
cd stellar-trust-circles
npm install
cp .env.example .env
```

Edit `.env`:
```
STELLAR_SECRET_KEY=SXXX...   # your testnet secret key
CONTRACT_ID=                  # leave blank until you deploy
STELLAR_NETWORK=testnet
```

### Fund your testnet account

```bash
curl "https://friendbot.stellar.org?addr=YOUR_PUBLIC_KEY"
```

### Build and deploy the contract locally

```bash
npm run build:contract
npm run deploy:testnet
# Copy the output contract ID into your .env as CONTRACT_ID
```

### Run the CLI

```bash
node cli.js status
node cli.js create --name "Test Circle" --amount 5 --cycle weekly --members ADDR1,ADDR2
```

---

## How to contribute

1. Check the [open issues](../../issues) — look for `good first issue` or `help wanted` labels
2. Comment on the issue to claim it
3. Fork the repo and create a branch: `git checkout -b feat/your-feature`
4. Make your changes
5. Open a pull request with a clear description

---

## Project structure

```
contracts/trust_circle/src/lib.rs   Soroban smart contract (Rust)
src/stellar.js                       Stellar SDK integration (JS)
cli.js                               CLI tool
```

---

## Areas that need help

- **Mobile UI** — React Native frontend for the contract
- **Social verification** — vouching system for member trust
- **Governance** — on-chain voting for circle rule changes.
- **Tests** — Soroban contract unit tests.
- **Docs** — tutorials for non-technical users.

---

## Code of conduct

Be kind and constructive. This project is for communities who depend on financial tools that work — keep that in mind