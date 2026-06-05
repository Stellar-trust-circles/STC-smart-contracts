#![cfg(test)]

use super::*;
use soroban_sdk::{
    testutils::{Address as _},
    token::{Client as TokenClient, StellarAssetClient},
    Address, Env, String, Vec,
};

fn setup_env() -> (Env, TrustCircleClient<'static>, Address, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();

    // Register a real mock USDC token contract
    let usdc_admin = Address::generate(&env);
    let usdc_contract = env.register_stellar_asset_contract_v2(usdc_admin.clone());
    let usdc = usdc_contract.address();

    // Mint USDC to admin and member2 so they can contribute
    let admin = Address::generate(&env);
    let member2 = Address::generate(&env);

    StellarAssetClient::new(&env, &usdc).mint(&admin, &1_000_000_000);
    StellarAssetClient::new(&env, &usdc).mint(&member2, &1_000_000_000);

    // Register Trust Circle contract
    let contract_id = env.register(TrustCircle, ());
    let client = TrustCircleClient::new(&env, &contract_id);

    let mut members = Vec::new(&env);
    members.push_back(admin.clone());
    members.push_back(member2.clone());

    client.create_circle(
        &admin,
        &String::from_str(&env, "Test Circle"),
        &usdc,
        &members,
        &100_000_000i128,
        &604800u64,
    );

    (env, client, admin, member2, usdc)
}

/// Circle should be active on cycle 1 right after creation
#[test]
fn test_create_circle() {
    let (_env, client, _admin, _member2, _usdc) = setup_env();

    let circle = client.get_circle();

    assert_eq!(circle.current_cycle, 1, "Should start on cycle 1");
    assert!(circle.is_active, "Circle should be active after creation");
    assert_eq!(circle.payout_index, 0, "Payout index should start at 0");
    assert_eq!(circle.contribution_amount, 100_000_000);
    assert_eq!(circle.members.len(), 2, "Should have 2 members");
}

/// A member should be able to contribute and have it recorded on-chain
#[test]
fn test_contribute() {
    let (_env, client, admin, _member2, _usdc) = setup_env();

    client.contribute(&admin);

    assert!(
        client.has_contributed(&admin, &1u32),
        "Admin should show as contributed for cycle 1"
    );

    let rep = client.get_reputation(&admin);
    assert_eq!(rep, 10, "Reputation should increase by 10");
}