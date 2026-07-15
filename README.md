cat > /mnt/user-data/outputs/contracts-README.md << 'ENDOFFILE'
<div align="center">

# STC Smart Contracts

**Soroban smart contracts powering the Stellar Trust Circles protocol**

*Decentralized rotating savings groups — built on Stellar*

[![CI](https://github.com/Stellar-trust-circles/contracts/actions/workflows/ci.yml/badge.svg)](https://github.com/Stellar-trust-circles/contracts/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Built on Stellar](https://img.shields.io/badge/Built%20on-Stellar-7C3AED)](https://stellar.org)
[![Soroban SDK](https://img.shields.io/badge/Soroban%20SDK-v25-0F6E56)](https://soroban.stellar.org)
[![Tests](https://img.shields.io/badge/Tests-30%20passing-0F6E56)](#testing)

</div>

---

## What is Trust Circles?

Rotating savings groups — known as *ajo* in Nigeria, *chama* in Kenya, *tanda* in Latin America, and *susu* in the Caribbean — are one of the oldest and most trusted forms of community finance. A fixed group of people contribute a set amount on a schedule; each cycle one member receives the full pooled amount, rotating until everyone has received once.

This repository contains the **Soroban smart contract** that replaces the trusted human treasurer at the center of these groups. The contract:

- Holds USDC contributions in escrow — no individual controls the pot
- Automates payout rotation based on the member order set at creation
- Records every contribution and payout permanently on-chain
- Tracks on-chain reputation for every participating address
- Enforces social vouching for new member admission
- Executes governance votes that change circle rules

---

## Repository Structure

```
STC-smart-contracts/
├── .github/
│   └── workflows/
│       └── ci.yml                    # Build, test, and lint on every push
├── contracts/
│   └── trust_circle/
│       ├── Cargo.toml                # Contract package — soroban-sdk v25
│       ├── Makefile                  # Per-contract build shortcuts
│       └── src/
│           ├── lib.rs                # Full contract logic (13 functions)
│           └── test.rs               # 30 unit tests
├── .clippy.toml                      # Clippy lint configuration
├── .gitignore
├── Cargo.lock
├── Cargo.toml                        # Workspace config
├── CONTRIBUTING.md
├── LICENSE
└── README.md
```

---

## Contract Overview

One contract deployment = one Trust Circle. The contract is written in Rust using the Soroban SDK and compiled to WebAssembly. All state is stored in Soroban instance storage.

### Storage keys

| Key | Type | Purpose |
|-----|------|---------|
| `Circle` | `Circle` struct | Full circle state — members, cycle, deadlines, active status |
| `Contributions` | `Map<(Address, u32), Contribution>` | Every contribution keyed by `(member, cycle)` |
| `Reputation(Address)` | `u32` | Per-address reputation score, persistent across circles |
| `Vouches(Address)` | `Vec<Address>` | List of addresses that have vouched for a newcomer |
| `Proposals` | `Map<u32, Proposal>` | All governance proposals, open and executed |
| `NextProposalId` | `u32` | Monotonically incrementing proposal ID counter |

### Contract functions

#### Write functions (require signed transaction)

| Function | Parameters | Description |
|----------|-----------|-------------|
| `create_circle` | `admin`, `name`, `usdc_token`, `members`, `contribution_amount`, `cycle_length_secs` | Initializes a new savings circle. Sets the rotation order, contribution rules, and first cycle deadline. Minimum 2 members. |
| `contribute` | `member` | Transfers `contribution_amount` USDC from the member's wallet into contract escrow. Records the contribution on-chain and awards +10 reputation. Panics if deadline has passed or member already contributed this cycle. |
| `release_payout` | `caller` | Tallies all contributions for the current cycle, penalizes missed members (−20 reputation, saturating at 0), sends the full pot to the next member in rotation, and advances the cycle. Admin can call at any time; any member can call after the deadline. |
| `restart_circle` | `admin` | Reactivates a completed circle (all members have received once) for a new rotation. Admin only. |
| `vouch` | `voucher`, `newcomer` | Records an on-chain endorsement of a newcomer's trustworthiness. Requires the voucher to have a reputation score of at least 50. Prevents double vouching. |
| `propose` | `proposer`, `proposal_type` | Creates a new governance proposal. Any circle member can propose. Returns the new proposal ID. |
| `vote` | `voter`, `proposal_id`, `vote_yes` | Casts a yes or no vote on an open proposal. One vote per member per proposal. |
| `execute_proposal` | `caller`, `proposal_id` | Applies a passed proposal's changes to the circle. Requires `votes_yes > total_members / 2`. Marks the proposal as executed. |

#### Read functions (free — no transaction required)

| Function | Returns | Description |
|----------|---------|-------------|
| `get_circle` | `Circle` | Full current circle state |
| `get_reputation` | `u32` | An address's on-chain reputation score, defaulting to 0 |
| `has_contributed` | `bool` | Whether a member has contributed in a given cycle |
| `get_vouches` | `u32` | How many vouches an address has received |
| `get_proposal` | `Proposal` | Full detail of a governance proposal by ID |

### Governance proposal types

| Variant | Value type | Effect when executed |
|---------|-----------|----------------------|
| `ChangeAmount(i128)` | New amount in stroops | Updates `contribution_amount` for all future cycles |
| `ChangeCycleLength(u64)` | New length in seconds | Updates `cycle_length_secs` from next cycle |
| `AddMember(Address)` | Stellar address | Appends address to the rotation |
| `RemoveMember(Address)` | Stellar address | Removes address from the rotation |

### On-chain events

| Symbol | Emitted by | Payload |
|--------|-----------|---------|
| `circle_crtd` | `create_circle` | Circle name |
| `contributed` | `contribute` | `(member, cycle)` |
| `payout_sent` | `release_payout` | `(recipient, amount, cycle)` |
| `completed` | `release_payout` | Final cycle number |
| `restarted` | `restart_circle` | Current cycle number |
| `vouched` | `vouch` | `(voucher, newcomer)` |
| `proposed` | `propose` | `(proposer, proposal_id)` |
| `voted` | `vote` | `(voter, proposal_id, vote_yes)` |
| `executed` | `execute_proposal` | `proposal_id` |

### Reputation system

| Action | Change |
|--------|--------|
| Contribute on time | `+10` points |
| Miss a contribution | `−20` points (saturates at 0, never negative) |

| Score | Tier |
|-------|------|
| 0 – 49 | New Member |
| 50 – 99 | Building Trust |
| 100+ | Trusted |

Reputation is stored per Stellar address and persists across all circles — it is not reset when a circle completes.

---

## Deployed Contracts

| Network | Contract ID | Status |
|---------|------------|--------|
| Testnet | `CANM5X47IG3AM5JDG6DVGZ24B3RLBNT5653CXRUEUDWF6JERO4YEX6ZS` | Active |
| Mainnet | — | Pending security audit |

> **Note:** Mainnet deployment is explicitly gated on a completed third-party security audit. Do not send real funds to any Mainnet deployment of this contract until an audit report is published in this repository.

---

## Prerequisites

| Tool | Version | Install |
|------|---------|---------|
| Rust | 1.84.0+ | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| WASM target | — | `rustup target add wasm32v1-none` |
| Stellar CLI | 26.0.0+ | `curl -fsSL https://github.com/stellar/stellar-cli/raw/main/install.sh \| sh -s -- --install-deps` |

---

## Getting Started

### 1. Clone and verify the setup

```bash
git clone https://github.com/Stellar-trust-circles/STC-smart-contracts
cd STC-smart-contracts
stellar --version        # should print v26.x.x
rustc --version          # should print 1.84.x or higher
```

### 2. Configure Testnet

```bash
stellar network add \
  --rpc-url https://soroban-testnet.stellar.org:443 \
  --network-passphrase "Test SDF Network ; September 2015" \
  testnet
```

### 3. Generate and fund a keypair

```bash
stellar keys generate alice --network testnet
curl "https://friendbot.stellar.org?addr=$(stellar keys address alice)"
```

### 4. Build the contract

```bash
stellar contract build
# Output: target/wasm32v1-none/release/trust_circle.wasm
```

### 5. Run the test suite

```bash
cargo test
# Running 30 tests — all should pass
```

### 6. Deploy to Testnet

```bash
stellar contract deploy \
  --wasm target/wasm32v1-none/release/trust_circle.wasm \
  --source alice \
  --network testnet \
  --alias trust_circle
# Copy the printed contract ID (starts with C)
```

### 7. Invoke a function

```bash
# Create a circle
stellar contract invoke \
  --id trust_circle \
  --source alice \
  --network testnet \
  -- \
  create_circle \
  --admin $(stellar keys address alice) \
  --name "My Circle" \
  --usdc_token GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5 \
  --members "[\"$(stellar keys address alice)\",\"SECOND_MEMBER_ADDRESS\"]" \
  --contribution_amount 100000000 \
  --cycle_length_secs 604800

# Read circle state (free)
stellar contract invoke \
  --id trust_circle \
  --source alice \
  --network testnet \
  -- \
  get_circle
```

---

## Testing

The test suite lives in `contracts/trust_circle/src/test.rs` and covers **30 tests** across the full contract surface.

### Test categories

**Core circle (5 tests)**
- `test_create_circle` — verifies initialization state
- `test_contribute` — records contribution and awards reputation
- `test_cannot_contribute_twice_in_same_cycle` — enforces single contribution per cycle
- `test_payout_rotation` — advances cycle and payout index correctly
- `test_missed_contribution_penalises_reputation` — applies reputation penalty

**Vouching (5 tests)**
- `test_new_address_has_zero_vouches`
- `test_vouch_with_low_reputation_fails` — reputation gate enforced
- `test_vouch_with_sufficient_reputation_succeeds` — successful vouch flow
- `test_cannot_vouch_twice` — duplicate vouch rejected
- `test_multiple_vouches_accumulate` — vouch count increments per voucher

**Governance — proposals (5 tests)**
- `test_propose_creates_proposal` — proposal stored with correct ID
- `test_proposal_ids_increment` — sequential IDs across proposals
- `test_propose_rejects_non_member` — membership gate enforced
- `test_multiple_proposals` — multiple open proposals coexist
- `test_rejected_proposal_no_change` — failed proposal leaves state unchanged

**Governance — voting (5 tests)**
- `test_vote_yes` / `test_vote_no` / `test_both_members_vote_yes`
- `test_cannot_double_vote` — one vote per member enforced
- `test_vote_rejects_non_member` — non-member cannot vote

**Governance — execution (10 tests)**
- `test_execute_rejects_insufficient_votes` / `test_execute_rejects_zero_votes`
- `test_cannot_execute_twice` / `test_cannot_vote_on_executed_proposal`
- `test_execute_change_amount` / `test_execute_change_cycle_length`
- `test_execute_add_member` / `test_execute_remove_member`
- `test_execute_marks_proposal_executed`
- `test_execute_by_non_member_succeeds_if_votes_pass`

### Test infrastructure

All tests use a shared `setup_env()` helper that:
1. Creates a Soroban test environment with `mock_all_auths()`
2. Registers a real mock USDC token via `register_stellar_asset_contract_v2`
3. Mints test USDC to all members and the contract
4. Registers and initializes the Trust Circle contract with a 2-member default circle

```bash
cargo test                         # Run all 30 tests
cargo test test_vouch              # Run vouching tests only
cargo test -- --nocapture          # Show println! output
```

---

## CI Pipeline

Every push and pull request to `main` runs two parallel jobs:

**Build and test job:**
1. Installs `libdbus-1-dev`, `libudev-dev`, `pkg-config` system dependencies
2. Installs Rust stable with `wasm32v1-none` target
3. Caches Cargo registry
4. Installs Stellar CLI via the official install script
5. Builds the contract with `stellar contract build`
6. Runs `cargo test --verbose`

**Lint job:**
1. Same system dependencies and Rust install
2. Runs `cargo clippy -- -D warnings` — any warning is a build failure

No pull request may be merged with a failing CI run.

---

## USDC Addresses

| Network | USDC Asset Issuer |
|---------|------------------|
| Testnet | `GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5` |
| Mainnet | `GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN` |

---

## Why Stellar?

| Need | How Stellar meets it |
|------|----------------------|
| Sub-cent fees | A $5 weekly contribution costs less than $0.001 in fees — viable at any amount |
| 3–5 second finality | Payout recipients see funds in the same session |
| Native USDC | No price volatility risk — members save and receive in dollars |
| Soroban smart contracts | Trustless rotation and governance logic enforced on-chain |
| Open SDK ecosystem | JavaScript and Python SDKs available — see the `documents` repo |

---

## Organisation Repositories

| Repo | Description |
|------|-------------|
| **STC-smart-contracts** (this repo) | Soroban contracts — Rust |
| [frontend](https://github.com/Stellar-trust-circles/frontend) | React + TypeScript web interface |
| [documents](https://github.com/Stellar-trust-circles/documents) | Docs, JavaScript SDK, Python SDK, CLI tool, code examples |

---

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for setup instructions, branch naming conventions, commit message format, and the review checklist every PR must pass.

Issues tagged [`good first issue`](../../issues?q=label%3A%22good+first+issue%22) are scoped to single functions with complete acceptance criteria — a great place to start.

---

## License

MIT — see [LICENSE](LICENSE)
