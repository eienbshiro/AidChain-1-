# AidChain

> Instant, transparent disaster relief distribution on Stellar — every peso tracked, no middlemen.

---

## Problem

After Typhoon Odette, a family in Tacloban waits 3 weeks for DSWD cash aid because paper-based beneficiary verification and multi-layer intermediaries delay delivery — and a portion of funds never arrives. Rural families travel hours to collect aid that may have been diverted before it reaches them.

## Solution

AidChain lets NGOs upload a verified beneficiary list to a Soroban smart contract. When a disaster is declared on-chain, XLM is distributed directly to each verified family's Stellar wallet within minutes — not weeks. Every registration, declaration, and payment is permanently recorded as a public on-chain event, making fund diversion impossible and enabling real-time audit by anyone.

**Why Stellar is essential:** Stellar's near-zero fees (< $0.00001/tx), 5-second finality, and native XLM transfers make mass micro-distributions economically viable. Soroban's smart contract logic enforces the no-duplicate, gate-before-distribute rules that prevent corruption without a trusted third party.

---

## Suggested MVP Timeline

| Day | Milestone |
|-----|-----------|
| 1   | Contract deployed to testnet, `initialize` + `add_beneficiary` working |
| 2   | `declare_disaster` + `distribute_aid` wired and tested |
| 3   | Front-end: NGO dashboard (add beneficiaries, declare, trigger distribution) |
| 4   | Front-end: Beneficiary lookup + public audit trail |
| 5   | Demo polish, testnet end-to-end walkthrough |

---

## Stellar Features Used

| Feature | How AidChain uses it |
|---|---|
| **Soroban smart contracts** | Core logic: beneficiary registry, disaster gate, anti-duplication, audit events |
| **XLM transfers** | Direct wallet-to-wallet aid distribution funded from contract balance |
| **On-chain events** | Every `add_beneficiary`, `declare_disaster`, and `distribute_aid` emits a public event |
| **Clawback / Compliance** | (Optional) NGO can reclaim undistributed funds after relief period ends |

---

## Vision and Purpose

Disaster relief is one of the most fraud-prone aid channels in the world. AidChain's goal is to make it impossible for intermediaries to siphon funds — not by adding bureaucracy, but by removing humans from the transfer layer entirely. The Soroban contract is the cashier. The Stellar ledger is the audit report. Every family's wallet is the bank.

Long-term vision: any NGO globally can deploy AidChain, upload a beneficiary CSV, and distribute verified aid within minutes of a disaster declaration — with an immutable, publicly verifiable payment history.

---

## Prerequisites

- **Rust toolchain** — Install via [rustup](https://rustup.rs/)
  ```bash
  rustup target add wasm32-unknown-unknown
  ```

- **Stellar CLI** (formerly Soroban CLI) v22+
  ```bash
  cargo install --locked stellar-cli --features opt
  ```

- **Testnet account** — Generate and fund via Stellar Friendbot:
  ```bash
  stellar keys generate --global alice --network testnet
  stellar keys fund alice --network testnet
  ```

---

## Build

```bash
# Clone the repository
git clone https://github.com/your-org/aid_chain
cd aid_chain

# Build the Wasm contract (optimized for deployment)
stellar contract build
```

Output: `target/wasm32-unknown-unknown/release/aid_chain.wasm`

---

## Test

```bash
# Run all 3 tests
cargo test

# Run with output (useful for debugging)
cargo test -- --nocapture
```

Expected output:
```
running 3 tests
test test::test_happy_path_register_declare_and_distribute ... ok
test test::test_duplicate_beneficiary_is_rejected ... ok
test test::test_storage_state_after_registration ... ok

test result: ok. 3 passed; 0 failed
```

---

## Deploy to Testnet

```bash
# Step 1: Deploy the compiled Wasm contract
stellar contract deploy \
  --wasm target/wasm32-unknown-unknown/release/aid_chain.wasm \
  --source alice \
  --network testnet

# Note the returned CONTRACT_ID — you'll need it for all subsequent calls

# Step 2: Get the XLM native token contract address on testnet
stellar contract id asset \
  --asset native \
  --network testnet

# Step 3: Initialize the contract
stellar contract invoke \
  --id <CONTRACT_ID> \
  --source alice \
  --network testnet \
  -- initialize \
  --admin <ADMIN_STELLAR_ADDRESS> \
  --token_id <NATIVE_TOKEN_CONTRACT_ID>
```

---

## Sample CLI Invocations

### Register a beneficiary
```bash
stellar contract invoke \
  --id <CONTRACT_ID> \
  --source alice \
  --network testnet \
  -- add_beneficiary \
  --wallet GBENEXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX \
  --aid_amount 50000000
# 50000000 stroops = 5 XLM
```

### Declare an active disaster
```bash
stellar contract invoke \
  --id <CONTRACT_ID> \
  --source alice \
  --network testnet \
  -- declare_disaster
```

### Distribute aid to a verified beneficiary
```bash
stellar contract invoke \
  --id <CONTRACT_ID> \
  --source alice \
  --network testnet \
  -- distribute_aid \
  --beneficiary_wallet GBENEXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX
```

### Check if a beneficiary has received aid
```bash
stellar contract invoke \
  --id <CONTRACT_ID> \
  --source alice \
  --network testnet \
  -- get_beneficiary \
  --wallet GBENEXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX
```

### Check disaster status
```bash
stellar contract invoke \
  --id <CONTRACT_ID> \
  --source alice \
  --network testnet \
  -- is_disaster_active
```

### Check total beneficiaries registered
```bash
stellar contract invoke \
  --id <CONTRACT_ID> \
  --source alice \
  --network testnet \
  -- get_beneficiary_count
```

---

## Contract Function Reference

| Function | Caller | Description |
|---|---|---|
| `initialize(admin, token_id)` | Deployer (once) | Sets up contract with NGO admin and XLM token |
| `add_beneficiary(wallet, aid_amount)` | Admin | Registers a verified beneficiary wallet |
| `declare_disaster()` | Admin | Opens the distribution gate |
| `distribute_aid(beneficiary_wallet)` | Admin | Transfers XLM to one beneficiary |
| `get_beneficiary(wallet)` | Anyone | Returns beneficiary record and distribution status |
| `is_disaster_active()` | Anyone | Returns whether distribution is currently open |
| `get_beneficiary_count()` | Anyone | Returns total number of registered beneficiaries |

---

## Security Design

- **Admin-only writes** — `add_beneficiary`, `declare_disaster`, and `distribute_aid` all call `admin.require_auth()`, so only the NGO's signing key can mutate state
- **One-time gate** — `initialize()` panics on a second call, preventing re-initialization attacks
- **No double-distribution** — Once `distributed = true`, no further XLM can be sent to that wallet
- **No double-registration** — Duplicate wallet addresses are rejected at registration time
- **Persistent storage** — Beneficiary records use `storage().persistent()` so they survive ledger archiving

---

## Reference

- Deploy guide: [https://github.com/armlynobinguar/Stellar-Bootcamp-2026](https://github.com/armlynobinguar/Stellar-Bootcamp-2026)
- Full-stack example: [https://github.com/armlynobinguar/community-treasury](https://github.com/armlynobinguar/community-treasury)
- Soroban docs: [https://developers.stellar.org/docs/build/smart-contracts](https://developers.stellar.org/docs/build/smart-contracts)

---

## Deployed Contract Link
[1] https://stellar.expert/explorer/testnet/tx/3f40a3fc37af975ef476f022bc66f885eba3f798c8f8525996039348516aa764
[2] https://lab.stellar.org/r/testnet/contract/CDEGGI5VJTU46VHEJS2WTPJHZQ3TKOHNS6CQ4YNWPWS3DSNCJ4LMTWQG

## License

MIT License

Copyright (c) 2026 AidChain Contributors

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.