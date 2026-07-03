#![cfg(test)]

use super::*;
use soroban_sdk::{
    testutils::Address as _,
    token::StellarAssetClient,
    Address, Env, String, Vec,
};

// ── Test helpers ──────────────────────────────────────────────────────────────

fn setup_env() -> (Env, TrustCircleClient<'static>, Address, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();

    // Register a real mock USDC token contract
    let usdc_admin = Address::generate(&env);
    let usdc_contract = env.register_stellar_asset_contract_v2(usdc_admin.clone());
    let usdc = usdc_contract.address();

    let admin = Address::generate(&env);
    let member2 = Address::generate(&env);

    // Mint plenty of USDC to all parties
    StellarAssetClient::new(&env, &usdc).mint(&admin, &10_000_000_000);
    StellarAssetClient::new(&env, &usdc).mint(&member2, &10_000_000_000);

    // Register Trust Circle contract
    let contract_id = env.register(TrustCircle, ());
    let client = TrustCircleClient::new(&env, &contract_id);

    // Mint USDC to the contract so it can release payouts
    StellarAssetClient::new(&env, &usdc).mint(&contract_id, &10_000_000_000);

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

/// Build reputation for a member by contributing across N full rotations.
/// With 2 members, one full rotation = 2 cycles (each member receives once).
/// After each full rotation the circle closes — we restart it each time.
/// Each contribute gives +10 rep, so 5 contributions = 50 rep.
fn build_reputation(
    client: &TrustCircleClient,
    member: &Address,
    other: &Address,
    contributions_needed: u32,
) {
    for i in 0..contributions_needed {
        // contribute as the target member each cycle
        client.contribute(member);

        // If the other member hasn't contributed this cycle,
        // contribute as them too so payout can be released
        // (we only need one member to contribute for payout to work
        //  but both need to so the circle stays healthy)
        client.contribute(other);
        client.release_payout(member);

        // After every 2 cycles the circle completes (2 members = 2 payouts = done)
        // Restart it so we can keep going
        let circle = client.get_circle();
        if !circle.is_active {
            client.restart_circle(member);
        }

        let _ = i; // suppress unused warning
    }
}

// ── Core tests ────────────────────────────────────────────────────────────────

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

/// Cannot contribute twice in the same cycle
#[test]
#[should_panic(expected = "Already contributed this cycle")]
fn test_cannot_contribute_twice_in_same_cycle() {
    let (_env, client, admin, _member2, _usdc) = setup_env();

    client.contribute(&admin);
    client.contribute(&admin); // should panic
}

/// After both members contribute and payout is released,
/// cycle should advance and payout index should move to member 2
#[test]
fn test_payout_rotation() {
    let (_env, client, admin, member2, _usdc) = setup_env();

    client.contribute(&admin);
    client.contribute(&member2);
    client.release_payout(&admin);

    let circle = client.get_circle();
    assert_eq!(circle.current_cycle, 2, "Cycle should advance to 2");
    assert_eq!(circle.payout_index, 1, "Payout index should advance to member 2");
}

/// A member who misses a contribution should have their reputation penalised
#[test]
fn test_missed_contribution_penalises_reputation() {
    let (_env, client, admin, member2, _usdc) = setup_env();

    // Only admin contributes — member2 misses
    client.contribute(&admin);
    client.release_payout(&admin);

    let rep = client.get_reputation(&member2);
    assert_eq!(rep, 0, "Missed member reputation should saturate at 0");

    let admin_rep = client.get_reputation(&admin);
    assert_eq!(admin_rep, 10, "Admin should have 10 reputation points");
}

// ── Vouching tests ────────────────────────────────────────────────────────────

/// A new address should start with 0 vouches
#[test]
fn test_new_address_has_zero_vouches() {
    let (env, client, _admin, _member2, _usdc) = setup_env();
    let newcomer = Address::generate(&env);

    let count = client.get_vouches(&newcomer);
    assert_eq!(count, 0, "New address should have 0 vouches");
}

/// A member with low reputation cannot vouch
#[test]
#[should_panic(expected = "Need reputation >= 50 to vouch for others")]
fn test_vouch_with_low_reputation_fails() {
    let (env, client, admin, _member2, _usdc) = setup_env();
    let newcomer = Address::generate(&env);

    // Admin has 0 rep — should panic
    client.vouch(&admin, &newcomer);
}

/// A member with sufficient reputation (>= 50) can vouch successfully
#[test]
fn test_vouch_with_sufficient_reputation_succeeds() {
    let (env, client, admin, member2, _usdc) = setup_env();
    let newcomer = Address::generate(&env);

    // 5 contributions x 10 rep each = 50 rep
    // With 2 members: 2 cycles per rotation, 3 rotations needed
    // rotation 1: cycles 1+2, rotation 2: cycles 3+4, then cycle 5
    build_reputation(&client, &admin, &member2, 5);

    let rep = client.get_reputation(&admin);
    assert!(rep >= 50, "Admin should have at least 50 rep, got {}", rep);

    client.vouch(&admin, &newcomer);

    let count = client.get_vouches(&newcomer);
    assert_eq!(count, 1, "Newcomer should have 1 vouch");
}

/// Cannot vouch for the same address twice
#[test]
#[should_panic(expected = "Already vouched for this address")]
fn test_cannot_vouch_twice() {
    let (env, client, admin, member2, _usdc) = setup_env();
    let newcomer = Address::generate(&env);

    build_reputation(&client, &admin, &member2, 5);

    // First vouch succeeds
    client.vouch(&admin, &newcomer);

    // Second vouch for same newcomer should panic
    client.vouch(&admin, &newcomer);
}

/// Multiple different members can vouch for the same newcomer
#[test]
fn test_multiple_vouches_accumulate() {
    let (env, client, admin, member2, _usdc) = setup_env();
    let newcomer = Address::generate(&env);

    // Build rep for both admin and member2
    build_reputation(&client, &admin, &member2, 5);

    let admin_rep = client.get_reputation(&admin);
    let member2_rep = client.get_reputation(&member2);
    assert!(admin_rep >= 50, "Admin should have >= 50 rep");
    assert!(member2_rep >= 50, "Member2 should have >= 50 rep");

    // Both vouch for the newcomer
    client.vouch(&admin, &newcomer);
    client.vouch(&member2, &newcomer);

    let count = client.get_vouches(&newcomer);
    assert_eq!(count, 2, "Newcomer should have 2 vouches");
}

// ── Governance tests ──────────────────────────────────────────────────────────

/// A member can create a proposal and it is stored correctly
#[test]
fn test_propose_creates_proposal() {
    let (_env, client, admin, _member2, _usdc) = setup_env();

    let new_amount = 200_000_000i128;
    let id = client.propose(&admin, &ProposalType::ChangeAmount(new_amount));

    assert_eq!(id, 0, "First proposal should have ID 0");

    let proposal = client.get_proposal(&id);
    assert_eq!(proposal.id, 0);
    assert_eq!(proposal.proposer, admin);
    assert_eq!(proposal.votes_yes, 0);
    assert_eq!(proposal.votes_no, 0);
    assert!(!proposal.executed);
    assert_eq!(proposal.voters.len(), 0);
}

/// Proposal IDs increment sequentially
#[test]
fn test_proposal_ids_increment() {
    let (_env, client, admin, _member2, _usdc) = setup_env();

    let id0 = client.propose(&admin, &ProposalType::ChangeAmount(200_000_000i128));
    let id1 = client.propose(&admin, &ProposalType::ChangeCycleLength(1209600u64));

    assert_eq!(id0, 0);
    assert_eq!(id1, 1);
}

/// A non-member cannot create a proposal
#[test]
#[should_panic(expected = "Only members can propose")]
fn test_propose_rejects_non_member() {
    let (env, client, _admin, _member2, _usdc) = setup_env();
    let outsider = Address::generate(&env);

    client.propose(&outsider, &ProposalType::ChangeAmount(200_000_000i128));
}

/// Voting yes tallies correctly
#[test]
fn test_vote_yes() {
    let (_env, client, admin, _member2, _usdc) = setup_env();

    let id = client.propose(&admin, &ProposalType::ChangeAmount(200_000_000i128));
    client.vote(&admin, &id, &true);

    let proposal = client.get_proposal(&id);
    assert_eq!(proposal.votes_yes, 1, "Should have 1 yes vote");
    assert_eq!(proposal.votes_no, 0, "Should have 0 no votes");
    assert_eq!(proposal.voters.len(), 1, "Should have 1 voter");
    assert!(proposal.voters.contains(&admin));
}

/// Voting no tallies correctly
#[test]
fn test_vote_no() {
    let (_env, client, admin, _member2, _usdc) = setup_env();

    let id = client.propose(&admin, &ProposalType::ChangeAmount(200_000_000i128));
    client.vote(&admin, &id, &false);

    let proposal = client.get_proposal(&id);
    assert_eq!(proposal.votes_yes, 0);
    assert_eq!(proposal.votes_no, 1);
}

/// Both members vote yes on a proposal
#[test]
fn test_both_members_vote_yes() {
    let (_env, client, admin, member2, _usdc) = setup_env();

    let id = client.propose(&admin, &ProposalType::ChangeAmount(200_000_000i128));
    client.vote(&admin, &id, &true);
    client.vote(&member2, &id, &true);

    let proposal = client.get_proposal(&id);
    assert_eq!(proposal.votes_yes, 2);
    assert_eq!(proposal.votes_no, 0);
}

/// A member cannot vote twice on the same proposal
#[test]
#[should_panic(expected = "Already voted on this proposal")]
fn test_cannot_double_vote() {
    let (_env, client, admin, _member2, _usdc) = setup_env();

    let id = client.propose(&admin, &ProposalType::ChangeAmount(200_000_000i128));
    client.vote(&admin, &id, &true);
    client.vote(&admin, &id, &true); // should panic
}

/// A non-member cannot vote
#[test]
#[should_panic(expected = "Only members can vote")]
fn test_vote_rejects_non_member() {
    let (env, client, admin, _member2, _usdc) = setup_env();
    let outsider = Address::generate(&env);

    let id = client.propose(&admin, &ProposalType::ChangeAmount(200_000_000i128));
    client.vote(&outsider, &id, &true);
}

/// Cannot execute a proposal that hasn't reached majority
#[test]
#[should_panic(expected = "Proposal has not reached majority")]
fn test_execute_rejects_insufficient_votes() {
    let (_env, client, admin, member2, _usdc) = setup_env();

    let id = client.propose(&admin, &ProposalType::ChangeAmount(200_000_000i128));
    // Only 1 yes vote out of 2 members — not a majority (>1)
    client.vote(&admin, &id, &true);
    client.execute_proposal(&member2, &id);
}

/// Cannot execute a proposal with zero votes
#[test]
#[should_panic(expected = "Proposal has not reached majority")]
fn test_execute_rejects_zero_votes() {
    let (_env, client, admin, _member2, _usdc) = setup_env();

    let id = client.propose(&admin, &ProposalType::ChangeAmount(200_000_000i128));
    client.execute_proposal(&admin, &id);
}

/// Cannot execute an already-executed proposal
#[test]
#[should_panic(expected = "Proposal already executed")]
fn test_cannot_execute_twice() {
    let (_env, client, admin, member2, _usdc) = setup_env();

    let id = client.propose(&admin, &ProposalType::ChangeAmount(200_000_000i128));
    client.vote(&admin, &id, &true);
    client.vote(&member2, &id, &true);
    client.execute_proposal(&admin, &id);
    client.execute_proposal(&admin, &id); // should panic
}

/// Cannot vote on an already-executed proposal (with 3rd member who hasn't voted)
#[test]
#[should_panic(expected = "Proposal already executed")]
fn test_cannot_vote_on_executed_proposal() {
    let (env, client, admin, member2, _usdc) = setup_env();
    let member3 = Address::generate(&env);

    // Add member3 via proposal so the circle has 3 members
    let add_id = client.propose(&admin, &ProposalType::AddMember(member3.clone()));
    client.vote(&admin, &add_id, &true);
    client.vote(&member2, &add_id, &true);
    client.execute_proposal(&admin, &add_id);

    // Create and execute a second proposal — member3 does NOT vote
    let id = client.propose(&admin, &ProposalType::ChangeAmount(200_000_000i128));
    client.vote(&admin, &id, &true);
    client.vote(&member2, &id, &true);
    client.execute_proposal(&admin, &id);

    // member3 tries to vote on the already-executed proposal — should panic
    client.vote(&member3, &id, &true);
}

/// Executing a ChangeAmount proposal updates contribution_amount
#[test]
fn test_execute_change_amount() {
    let (_env, client, admin, member2, _usdc) = setup_env();

    let new_amount = 200_000_000i128;
    let id = client.propose(&admin, &ProposalType::ChangeAmount(new_amount));
    client.vote(&admin, &id, &true);
    client.vote(&member2, &id, &true);
    client.execute_proposal(&admin, &id);

    let circle = client.get_circle();
    assert_eq!(circle.contribution_amount, new_amount);
}

/// Executing a ChangeCycleLength proposal updates cycle_length_secs
#[test]
fn test_execute_change_cycle_length() {
    let (_env, client, admin, member2, _usdc) = setup_env();

    let new_length = 1_209_600u64; // 2 weeks
    let id = client.propose(&admin, &ProposalType::ChangeCycleLength(new_length));
    client.vote(&admin, &id, &true);
    client.vote(&member2, &id, &true);
    client.execute_proposal(&admin, &id);

    let circle = client.get_circle();
    assert_eq!(circle.cycle_length_secs, new_length);
}

/// Executing an AddMember proposal adds the new member to the circle
#[test]
fn test_execute_add_member() {
    let (env, client, admin, member2, _usdc) = setup_env();
    let new_member = Address::generate(&env);

    let id = client.propose(&admin, &ProposalType::AddMember(new_member.clone()));
    client.vote(&admin, &id, &true);
    client.vote(&member2, &id, &true);
    client.execute_proposal(&admin, &id);

    let circle = client.get_circle();
    assert_eq!(circle.members.len(), 3, "Circle should now have 3 members");
    assert!(circle.members.contains(&new_member), "New member should be in the circle");
}

/// Executing a RemoveMember proposal removes the member from the circle
#[test]
fn test_execute_remove_member() {
    let (_env, client, admin, member2, _usdc) = setup_env();

    let id = client.propose(&admin, &ProposalType::RemoveMember(member2.clone()));
    client.vote(&admin, &id, &true);
    client.vote(&member2, &id, &true);
    client.execute_proposal(&admin, &id);

    let circle = client.get_circle();
    assert_eq!(circle.members.len(), 1, "Circle should now have 1 member");
    assert!(
        !circle.members.contains(&member2),
        "Removed member should not be in the circle"
    );
}

/// Executing a proposal marks it as executed
#[test]
fn test_execute_marks_proposal_executed() {
    let (_env, client, admin, member2, _usdc) = setup_env();

    let id = client.propose(&admin, &ProposalType::ChangeAmount(200_000_000i128));
    client.vote(&admin, &id, &true);
    client.vote(&member2, &id, &true);
    client.execute_proposal(&admin, &id);

    let proposal = client.get_proposal(&id);
    assert!(proposal.executed, "Proposal should be marked executed");
}

/// Multiple proposals can exist independently
#[test]
fn test_multiple_proposals() {
    let (_env, client, admin, member2, _usdc) = setup_env();

    let id0 = client.propose(&admin, &ProposalType::ChangeAmount(200_000_000i128));
    let id1 = client.propose(&member2, &ProposalType::ChangeCycleLength(1_209_600u64));

    // Vote and execute first proposal
    client.vote(&admin, &id0, &true);
    client.vote(&member2, &id0, &true);
    client.execute_proposal(&admin, &id0);

    // Second proposal is still open
    let p1 = client.get_proposal(&id1);
    assert!(!p1.executed, "Second proposal should not be executed yet");
    assert_eq!(p1.votes_yes, 0);

    // Now vote and execute it
    client.vote(&admin, &id1, &true);
    client.vote(&member2, &id1, &true);
    client.execute_proposal(&admin, &id1);

    let circle = client.get_circle();
    assert_eq!(circle.contribution_amount, 200_000_000);
    assert_eq!(circle.cycle_length_secs, 1_209_600);
}