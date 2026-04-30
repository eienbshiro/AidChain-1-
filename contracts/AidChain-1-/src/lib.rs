//! AidChain — Disaster Relief Distribution on Stellar
//!
//! This contract allows an NGO admin to:
//!   1. Register verified beneficiary wallets with a fixed aid amount
//!   2. Declare an active disaster event (the distribution gate)
//!   3. Distribute XLM directly to each verified beneficiary wallet
//!
//! All actions emit on-chain events, forming a tamper-proof public audit trail.

#![no_std]

use soroban_sdk::{
    contract, contractimpl, contracttype,
    symbol_short, token, Address, Env,
};

// ─── Data Structures ────────────────────────────────────────────────────────

/// Stores all relevant info for a single disaster relief beneficiary.
#[contracttype]
#[derive(Clone)]
pub struct BeneficiaryInfo {
    /// The beneficiary's Stellar wallet address
    pub wallet: Address,
    /// XLM aid amount in stroops (1 XLM = 10_000_000 stroops)
    pub aid_amount: i128,
    /// Whether aid has already been sent — prevents double distribution
    pub distributed: bool,
}

/// Storage keys for all contract state.
/// Enum variants map to specific slots in Soroban's key-value store.
#[contracttype]
pub enum DataKey {
    /// The NGO admin address — only this account can call privileged functions
    Admin,
    /// The XLM token contract address (Stellar native asset contract)
    TokenId,
    /// Whether a disaster event is currently active — gates all distributions
    DisasterActive,
    /// Per-beneficiary record, keyed by wallet address
    Beneficiary(Address),
    /// Running count of all registered beneficiaries (for dashboard display)
    BeneficiaryCount,
}

// ─── Contract ───────────────────────────────────────────────────────────────

#[contract]
pub struct AidChain;

#[contractimpl]
impl AidChain {
    // ── Setup ────────────────────────────────────────────────────────────────

    /// Initializes the AidChain contract.
    ///
    /// Must be called once immediately after deployment. Sets the NGO admin
    /// wallet and the XLM token contract address used for all distributions.
    /// Panics if called a second time to prevent re-initialization attacks.
    ///
    /// # Arguments
    /// * `admin`    — The NGO's Stellar wallet; receives all admin privileges
    /// * `token_id` — Address of the XLM native asset contract on this network
    pub fn initialize(env: Env, admin: Address, token_id: Address) {
        // Prevent re-initialization — once set, the admin cannot be overwritten
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("Contract already initialized");
        }

        // Admin must sign the initialization transaction
        admin.require_auth();

        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::TokenId, &token_id);
        env.storage().instance().set(&DataKey::DisasterActive, &false);
        env.storage().instance().set(&DataKey::BeneficiaryCount, &0u32);
    }

    // ── Beneficiary Management ───────────────────────────────────────────────

    /// Registers a verified beneficiary wallet and their allocated aid amount.
    ///
    /// Only the NGO admin can call this. Duplicate registrations for the same
    /// wallet are rejected — each family can only appear once on the list.
    /// Emits an `ADD_BEN` event for public audit transparency.
    ///
    /// # Arguments
    /// * `wallet`     — The beneficiary's Stellar wallet address
    /// * `aid_amount` — XLM amount in stroops to distribute to this wallet
    pub fn add_beneficiary(env: Env, wallet: Address, aid_amount: i128) {
        // Only the admin may register beneficiaries
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        // Reject duplicate registrations — each wallet can only be registered once
        if env.storage()
            .persistent()
            .has(&DataKey::Beneficiary(wallet.clone()))
        {
            panic!("Beneficiary already registered");
        }

        let info = BeneficiaryInfo {
            wallet: wallet.clone(),
            aid_amount,
            distributed: false,
        };

        // Persist the beneficiary record using persistent storage
        // (survives ledger archiving — important for long relief operations)
        env.storage()
            .persistent()
            .set(&DataKey::Beneficiary(wallet.clone()), &info);

        // Emit event — contributes to the public on-chain audit trail
        env.events()
            .publish((symbol_short!("ADD_BEN"), wallet), aid_amount);

        // Increment the total beneficiary count for dashboard display
        let count: u32 = env
            .storage()
            .instance()
            .get(&DataKey::BeneficiaryCount)
            .unwrap_or(0);
        env.storage()
            .instance()
            .set(&DataKey::BeneficiaryCount, &(count + 1));
    }

    // ── Disaster Event ───────────────────────────────────────────────────────

    /// Declares an active disaster event, enabling XLM distributions.
    ///
    /// Acts as a safety gate — no aid can be distributed until this is called.
    /// This mirrors real-world NGO protocols where distribution is only triggered
    /// after an official disaster declaration. Emits a `DISASTER` event.
    ///
    /// Only the NGO admin can call this.
    pub fn declare_disaster(env: Env) {
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        env.storage()
            .instance()
            .set(&DataKey::DisasterActive, &true);

        // Public event — signals to front-ends and monitors that distributions are open
        env.events()
            .publish((symbol_short!("DISASTER"),), true);
    }

    // ── Aid Distribution ─────────────────────────────────────────────────────

    /// Distributes XLM aid directly to a verified beneficiary wallet.
    ///
    /// This is the core MVP function. It:
    ///   1. Confirms a disaster is active (the distribution gate)
    ///   2. Confirms the beneficiary is on the verified list
    ///   3. Confirms aid has not already been sent (prevents double-distribution)
    ///   4. Transfers XLM from the contract's balance to the beneficiary's wallet
    ///   5. Marks the beneficiary as distributed and emits an `AID_SENT` event
    ///
    /// Only the NGO admin can trigger distributions.
    ///
    /// # Arguments
    /// * `beneficiary_wallet` — Address of the beneficiary to receive XLM aid
    pub fn distribute_aid(env: Env, beneficiary_wallet: Address) {
        // Gate 1: A disaster must be declared before any money moves
        let active: bool = env
            .storage()
            .instance()
            .get(&DataKey::DisasterActive)
            .unwrap_or(false);
        if !active {
            panic!("No active disaster declared");
        }

        // Gate 2: Only the admin may trigger distributions
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        // Load the beneficiary record — panic if wallet is not on the list
        let mut info: BeneficiaryInfo = env
            .storage()
            .persistent()
            .get(&DataKey::Beneficiary(beneficiary_wallet.clone()))
            .unwrap_or_else(|| panic!("Beneficiary not found"));

        // Gate 3: Prevent double-distribution to the same wallet
        if info.distributed {
            panic!("Aid already distributed to this beneficiary");
        }

        // Transfer XLM from this contract's balance to the beneficiary wallet.
        // The contract must hold enough XLM — typically funded by the NGO
        // prior to declaring the disaster.
        let token_id: Address = env
            .storage()
            .instance()
            .get(&DataKey::TokenId)
            .unwrap();
        let token_client = token::Client::new(&env, &token_id);
        token_client.transfer(
            &env.current_contract_address(), // from: the contract's own balance
            &beneficiary_wallet,             // to: the beneficiary's wallet
            &info.aid_amount,                // amount in stroops
        );

        // Mark as distributed — prevents any future claim attempt
        info.distributed = true;
        env.storage()
            .persistent()
            .set(&DataKey::Beneficiary(beneficiary_wallet.clone()), &info);

        // Emit the audit event — every distribution is permanently recorded on-chain
        env.events()
            .publish((symbol_short!("AID_SENT"), beneficiary_wallet), info.aid_amount);
    }

    // ── Read-Only Queries ────────────────────────────────────────────────────

    /// Returns the full beneficiary record for a given wallet.
    /// Used by front-end dashboards, NGO auditors, and beneficiaries themselves.
    pub fn get_beneficiary(env: Env, wallet: Address) -> BeneficiaryInfo {
        env.storage()
            .persistent()
            .get(&DataKey::Beneficiary(wallet))
            .unwrap_or_else(|| panic!("Beneficiary not found"))
    }

    /// Returns whether a disaster event is currently active.
    pub fn is_disaster_active(env: Env) -> bool {
        env.storage()
            .instance()
            .get(&DataKey::DisasterActive)
            .unwrap_or(false)
    }

    /// Returns the total number of registered beneficiaries.
    /// Useful for front-end counters and NGO reporting.
    pub fn get_beneficiary_count(env: Env) -> u32 {
        env.storage()
            .instance()
            .get(&DataKey::BeneficiaryCount)
            .unwrap_or(0)
    }
}

#[cfg(test)]

mod test;