use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token::StellarAssetClient,
    vec, Env, String,
};

fn setup_circle() -> (Env, Address, Address, Address, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger()
        .with_mut(|ledger| ledger.timestamp = 1_700_000_000);

    let contract_id = env.register(TrustCircle, ());
    let client = TrustCircleClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let member_one = Address::generate(&env);
    let member_two = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = env
        .register_stellar_asset_contract_v2(token_admin)
        .address();

    let asset_client = StellarAssetClient::new(&env, &token);
    asset_client.mint(&member_one, &1_000);
    asset_client.mint(&member_two, &1_000);

    client.create_circle(
        &admin,
        &String::from_str(&env, "Lagos Squad"),
        &token,
        &vec![&env, member_one.clone(), member_two.clone()],
        &100,
        &3_600,
    );

    (env, contract_id, admin, member_one, member_two, token)
}

#[test]
fn test_create_circle() {
    let (env, contract_id, admin, member_one, member_two, token) = setup_circle();
    let client = TrustCircleClient::new(&env, &contract_id);

    let circle = client.get_circle();

    assert!(circle.is_active);
    assert_eq!(circle.current_cycle, 1);
    assert_eq!(circle.payout_index, 0);
    assert_eq!(circle.admin, admin);
    assert_eq!(circle.usdc_token, token);
    assert_eq!(circle.members.len(), 2);
    assert_eq!(circle.members.get(0), Some(member_one));
    assert_eq!(circle.members.get(1), Some(member_two));
}

#[test]
fn test_contribute() {
    let (env, contract_id, _admin, member_one, _member_two, _token) = setup_circle();
    let client = TrustCircleClient::new(&env, &contract_id);

    client.contribute(&member_one);

    assert!(client.has_contributed(&member_one, &1));
    assert_eq!(client.get_reputation(&member_one), 10);
}

#[test]
fn test_cannot_contribute_twice_in_same_cycle() {
    let (env, contract_id, _admin, member_one, _member_two, _token) = setup_circle();
    let client = TrustCircleClient::new(&env, &contract_id);

    client.contribute(&member_one);
    let second_contribution = client.try_contribute(&member_one);

    assert!(second_contribution.is_err());
}

#[test]
fn test_payout_rotation() {
    let (env, contract_id, admin, member_one, member_two, token) = setup_circle();
    let client = TrustCircleClient::new(&env, &contract_id);
    let token_client = token::Client::new(&env, &token);

    client.contribute(&member_one);
    client.contribute(&member_two);
    client.release_payout(&admin);

    let circle = client.get_circle();
    assert_eq!(circle.current_cycle, 2);
    assert_eq!(circle.payout_index, 1);
    assert!(circle.is_active);
    assert_eq!(token_client.balance(&member_one), 1_100);
    assert_eq!(token_client.balance(&member_two), 900);
    assert_eq!(token_client.balance(&contract_id), 0);
}

#[test]
fn test_missed_contribution_penalises_reputation() {
    let (env, contract_id, admin, member_one, member_two, _token) = setup_circle();
    let client = TrustCircleClient::new(&env, &contract_id);

    client.contribute(&member_one);
    client.contribute(&member_two);
    client.release_payout(&admin);

    assert_eq!(client.get_reputation(&member_one), 10);
    assert_eq!(client.get_reputation(&member_two), 10);

    client.contribute(&member_one);
    client.release_payout(&admin);

    assert_eq!(client.get_reputation(&member_one), 20);
    assert_eq!(client.get_reputation(&member_two), 0);
}
