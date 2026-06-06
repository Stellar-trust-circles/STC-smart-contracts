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
    let (env, client, admin, member2, _usdc) = setup_env();

    client.contribute(&admin);
    client.contribute(&member2);

    // Advance ledger past the cycle deadline
    env.ledger().set(LedgerInfo {
        timestamp: 1_000_000 + 604_801, // just past 1 week
        protocol_version: 22,
        sequence_number: 200,
        network_id: Default::default(),
        base_reserve: 10,
        min_temp_entry_ttl: 16,
        min_persistent_entry_ttl: 100,
        max_entry_ttl: 6_312_000,
    });

    client.release_payout(&admin);

    let circle = client.get_circle();
    assert_eq!(circle.current_cycle, 2, "Cycle should advance to 2");
    assert_eq!(circle.payout_index, 1, "Payout index should advance to member 2");
}

/// A member who misses a contribution should have their
/// reputation penalised when payout is released
#[test]
fn test_missed_contribution_penalises_reputation() {
    let (env, client, admin, member2, _usdc) = setup_env();

    // Only admin contributes — member2 misses
    client.contribute(&admin);

    // Advance past deadline
    env.ledger().set(LedgerInfo {
        timestamp: 1_000_000 + 604_801,
        protocol_version: 22,
        sequence_number: 200,
        network_id: Default::default(),
        base_reserve: 10,
        min_temp_entry_ttl: 16,
        min_persistent_entry_ttl: 100,
        max_entry_ttl: 6_312_000,
    });

    client.release_payout(&admin);

    // member2 never contributed so should have been penalised
    // Starting from 0, saturating_sub(20) = 0
    let rep = client.get_reputation(&member2);
    assert_eq!(rep, 0, "Missed member reputation should saturate at 0");

    // Admin contributed on time so should have gained points
    let admin_rep = client.get_reputation(&admin);
    assert_eq!(admin_rep, 10, "Admin should have 10 reputation points");
}