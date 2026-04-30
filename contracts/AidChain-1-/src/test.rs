// AidChain Test Suite
//
// Exactly 3 tests:
//   Test 1 — Happy path: beneficiary registered, disaster declared, aid distributed
//   Test 2 — Edge case: duplicate beneficiary registration is rejected
//   Test 3 — State verification: storage reflects correct state after registration

use super::*;
use soroban_sdk::{
    testutils::Address as _,
    token, Address, Env,
};

// ─── Test Helpers ────────────────────────────────────────────────────────────

/// Sets up the full test environment:
///   - Creates an Env and mocks all auth (no real key signing needed in tests)
///   - Deploys the AidChain contract
///   - Deploys a mock XLM native asset contract (Stellar Asset Contract)
///   - Mints XLM into the AidChain contract so it can fund distributions
///   - Calls initialize() on the AidChain contract
///
/// Returns (env, client, admin_address, token_address)
fn setup() -> (Env, AidChainClient<'static>, Address, Address) {
    let env = Env::default();

    // mock_all_auths() bypasses signature verification in tests —
    // every require_auth() call will automatically succeed.
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let contract_id = env.register_contract(None, AidChain);
    let client = AidChainClient::new(&env, &contract_id);

    // Deploy a Stellar Asset Contract (SAC) to simulate the XLM native token
    let token_admin = Address::generate(&env);
    let sac = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_address = sac.address();

    // Mint 1,000 XLM (in stroops) to the AidChain contract balance.
    // In production, the NGO funds the contract before declaring a disaster.
    let stellar_asset_client = token::StellarAssetClient::new(&env, &token_address);
    stellar_asset_client.mint(&contract_id, &1_000_000_000i128); // 100 XLM = 1_000_000_000 stroops

    // Initialize the AidChain contract with admin and token address
    client.initialize(&admin, &token_address);

    (env, client, admin, token_address)
}

// ─── Tests ───────────────────────────────────────────────────────────────────

/// Test 1 (Happy Path)
///
/// Verifies the full end-to-end MVP flow:
///   1. Admin registers a beneficiary with an aid amount
///   2. Admin declares an active disaster
///   3. Admin distributes XLM to the beneficiary wallet
///   4. The beneficiary record is marked as distributed
///   5. The beneficiary's token balance has increased by the aid amount
#[test]
fn test_happy_path_register_declare_and_distribute() {
    let (env, client, _admin, token_address) = setup();

    let beneficiary = Address::generate(&env);
    let aid_amount: i128 = 50_000_000; // 5 XLM in stroops

    // Step 1: Register the beneficiary
    client.add_beneficiary(&beneficiary, &aid_amount);

    // Step 2: Declare the disaster — unlocks distributions
    client.declare_disaster();

    // Confirm the disaster flag is active
    assert!(
        client.is_disaster_active(),
        "Disaster should be active after declaration"
    );

    // Step 3: Distribute aid to the beneficiary
    client.distribute_aid(&beneficiary);

    // Step 4: Verify the beneficiary record is marked as distributed
    let info = client.get_beneficiary(&beneficiary);
    assert!(
        info.distributed,
        "Beneficiary should be marked as distributed after aid is sent"
    );
    assert_eq!(
        info.aid_amount, aid_amount,
        "Aid amount in storage should match the registered amount"
    );

    // Step 5: Verify the beneficiary's actual XLM token balance increased
    let token_client = token::Client::new(&env, &token_address);
    let balance = token_client.balance(&beneficiary);
    assert_eq!(
        balance, aid_amount,
        "Beneficiary's token balance should equal the distributed aid amount"
    );
}

/// Test 2 (Edge Case)
///
/// Verifies that registering the same wallet address twice is rejected.
/// This prevents a single family from claiming multiple aid slots
/// and protects the integrity of the beneficiary list.
#[test]
#[should_panic(expected = "Beneficiary already registered")]
fn test_duplicate_beneficiary_is_rejected() {
    let (env, client, _admin, _token) = setup();

    let beneficiary = Address::generate(&env);
    let aid_amount: i128 = 50_000_000; // 5 XLM in stroops

    // First registration succeeds
    client.add_beneficiary(&beneficiary, &aid_amount);

    // Second registration of the same wallet must panic
    // Expected panic message: "Beneficiary already registered"
    client.add_beneficiary(&beneficiary, &aid_amount);
}

/// Test 3 (State Verification)
///
/// Verifies that the contract's persistent storage accurately reflects
/// the correct state after a successful beneficiary registration:
///   - Wallet address is stored correctly
///   - Aid amount is stored correctly
///   - distributed flag starts as false (aid not yet sent)
///   - Beneficiary count increments correctly
#[test]
fn test_storage_state_after_registration() {
    let (env, client, _admin, _token) = setup();

    let beneficiary = Address::generate(&env);
    let aid_amount: i128 = 100_000_000; // 10 XLM in stroops

    // Confirm count starts at zero
    assert_eq!(
        client.get_beneficiary_count(),
        0,
        "Beneficiary count should start at 0"
    );

    // Register the beneficiary
    client.add_beneficiary(&beneficiary, &aid_amount);

    // Fetch the stored record and verify every field
    let info = client.get_beneficiary(&beneficiary);

    assert_eq!(
        info.wallet, beneficiary,
        "Stored wallet must match the registered address"
    );
    assert_eq!(
        info.aid_amount, aid_amount,
        "Stored aid amount must match the registered amount"
    );
    assert!(
        !info.distributed,
        "distributed flag must be false before any aid is sent"
    );

    // Verify the beneficiary count incremented
    assert_eq!(
        client.get_beneficiary_count(),
        1,
        "Beneficiary count should be 1 after one registration"
    );
}
