#![no_std]
#![allow(deprecated)]
use soroban_sdk::{
    contract, contractimpl, contracttype, token, Address, Env, Map, String, Symbol, Vec,
};

// ── Storage keys ──────────────────────────────────────────────────────────────

#[contracttype]
pub enum DataKey {
    Circle,
    Contributions,
    Reputation(Address),
    Vouches(Address),
}

// ── Data types ────────────────────────────────────────────────────────────────

#[contracttype]
#[derive(Clone)]
pub struct Circle {
    pub name: String,
    pub admin: Address,
    pub usdc_token: Address,
    pub contribution_amount: i128,
    pub members: Vec<Address>,
    pub current_cycle: u32,
    pub payout_index: u32,
    pub cycle_deadline: u64,
    pub cycle_length_secs: u64,
    pub is_active: bool,
}

#[contracttype]
#[derive(Clone)]
pub struct Contribution {
    pub member: Address,
    pub cycle: u32,
    pub amount: i128,
    pub timestamp: u64,
}

// ── Contract ──────────────────────────────────────────────────────────────────

#[contract]
pub struct TrustCircle;

#[contractimpl]
impl TrustCircle {
    /// Initialize a new savings circle.
    /// Called once by the circle creator.
    pub fn create_circle(
        env: Env,
        admin: Address,
        name: String,
        usdc_token: Address,
        members: Vec<Address>,
        contribution_amount: i128,
        cycle_length_secs: u64,
    ) {
        admin.require_auth();

        assert!(members.len() >= 2, "Need at least 2 members");
        assert!(contribution_amount > 0, "Contribution must be positive");
        assert!(cycle_length_secs >= 3600, "Cycle must be at least 1 hour");

        let now = env.ledger().timestamp();
        let circle = Circle {
            name: name.clone(),
            admin,
            usdc_token,
            contribution_amount,
            members,
            current_cycle: 1,
            payout_index: 0,
            cycle_deadline: now + cycle_length_secs,
            cycle_length_secs,
            is_active: true,
        };

        env.storage().instance().set(&DataKey::Circle, &circle);
        env.storage()
            .instance()
            .set(&DataKey::Contributions, &Map::<(Address, u32), Contribution>::new(&env));

        env.events().publish(
            (Symbol::new(&env, "circle_crtd"),),
            name,
        );
    }

    /// A member contributes their share for the current cycle.
    pub fn contribute(env: Env, member: Address) {
        member.require_auth();

        let circle: Circle = env.storage().instance().get(&DataKey::Circle).unwrap();
        assert!(circle.is_active, "Circle is not active");
        assert!(circle.members.contains(&member), "Not a circle member");

        let now = env.ledger().timestamp();
        assert!(now <= circle.cycle_deadline, "Cycle deadline has passed");

        let mut contributions: Map<(Address, u32), Contribution> = env
            .storage()
            .instance()
            .get(&DataKey::Contributions)
            .unwrap();

        let key = (member.clone(), circle.current_cycle);
        assert!(
            !contributions.contains_key(key.clone()),
            "Already contributed this cycle"
        );

        // Transfer USDC from member to this contract
        let token_client = token::Client::new(&env, &circle.usdc_token);
        token_client.transfer(
            &member,
            &env.current_contract_address(),
            &circle.contribution_amount,
        );

        // Record the contribution
        contributions.set(
            key,
            Contribution {
                member: member.clone(),
                cycle: circle.current_cycle,
                amount: circle.contribution_amount,
                timestamp: now,
            },
        );
        env.storage()
            .instance()
            .set(&DataKey::Contributions, &contributions);

        // Update reputation: +10 points per on-time contribution
        let rep_key = DataKey::Reputation(member.clone());
        let current_rep: u32 = env.storage().instance().get(&rep_key).unwrap_or(0);
        env.storage()
            .instance()
            .set(&rep_key, &(current_rep + 10));

        env.events().publish(
            (Symbol::new(&env, "contributed"),),
            (member, circle.current_cycle),
        );
    }

    /// Release the payout to the next member in rotation.
    /// Can be called by admin at any time, or by any member after the deadline.
    pub fn release_payout(env: Env, caller: Address) {
        caller.require_auth();

        let mut circle: Circle = env.storage().instance().get(&DataKey::Circle).unwrap();
        assert!(circle.is_active, "Circle is not active");

        let now = env.ledger().timestamp();
        assert!(
            now >= circle.cycle_deadline || caller == circle.admin,
            "Deadline not reached"
        );

        let contributions: Map<(Address, u32), Contribution> = env
            .storage()
            .instance()
            .get(&DataKey::Contributions)
            .unwrap();

        // Tally contributions and penalise missed members
        let mut total_contributed: i128 = 0;
        for member in circle.members.iter() {
            let key = (member.clone(), circle.current_cycle);
            if contributions.contains_key(key) {
                total_contributed += circle.contribution_amount;
            } else {
                let rep_key = DataKey::Reputation(member.clone());
                let rep: u32 = env.storage().instance().get(&rep_key).unwrap_or(0);
                env.storage()
                    .instance()
                    .set(&rep_key, &rep.saturating_sub(20));
            }
        }

        assert!(total_contributed > 0, "No contributions to pay out");

        // Send payout to the next recipient in rotation
        let recipient = circle.members.get(circle.payout_index).unwrap();
        let token_client = token::Client::new(&env, &circle.usdc_token);
        token_client.transfer(
            &env.current_contract_address(),
            &recipient,
            &total_contributed,
        );

        env.events().publish(
            (Symbol::new(&env, "payout_sent"),),
            (recipient.clone(), total_contributed, circle.current_cycle),
        );

        // Advance rotation
        circle.payout_index = (circle.payout_index + 1) % circle.members.len();
        circle.current_cycle += 1;
        circle.cycle_deadline = now + circle.cycle_length_secs;

        // If full rotation complete, mark circle as done
        if circle.payout_index == 0 {
            circle.is_active = false;
            env.events().publish(
                (Symbol::new(&env, "completed"),),
                circle.current_cycle,
            );
        }

        env.storage().instance().set(&DataKey::Circle, &circle);
    }

    /// View the current circle state (read-only).
    pub fn get_circle(env: Env) -> Circle {
        env.storage().instance().get(&DataKey::Circle).unwrap()
    }

    /// View a member's on-chain reputation score (read-only).
    pub fn get_reputation(env: Env, member: Address) -> u32 {
        env.storage()
            .instance()
            .get(&DataKey::Reputation(member))
            .unwrap_or(0)
    }

    /// Check if a member has contributed in a given cycle (read-only).
    pub fn has_contributed(env: Env, member: Address, cycle: u32) -> bool {
        let contributions: Map<(Address, u32), Contribution> = env
            .storage()
            .instance()
            .get(&DataKey::Contributions)
            .unwrap_or(Map::new(&env));
        contributions.contains_key((member, cycle))
    }

    /// Admin: restart a completed circle for another rotation.
    pub fn restart_circle(env: Env, admin: Address) {
        admin.require_auth();

        let mut circle: Circle = env.storage().instance().get(&DataKey::Circle).unwrap();
        assert!(circle.admin == admin, "Only admin can restart");
        assert!(!circle.is_active, "Circle is still active");

        circle.is_active = true;
        circle.payout_index = 0;
        circle.cycle_deadline = env.ledger().timestamp() + circle.cycle_length_secs;

        env.storage().instance().set(&DataKey::Circle, &circle);

        env.events().publish(
            (Symbol::new(&env, "restarted"),),
            circle.current_cycle,
        );
    }

    /// Vouch for a newcomer address.
    /// The voucher must have a reputation score of at least 50.
    /// A voucher can only vouch once per newcomer.
    pub fn vouch(env: Env, voucher: Address, newcomer: Address) {
        voucher.require_auth();

        // Check voucher has enough reputation
        let rep: u32 = env
            .storage()
            .instance()
            .get(&DataKey::Reputation(voucher.clone()))
            .unwrap_or(0);
        assert!(rep >= 50, "Need reputation >= 50 to vouch for others");

        // Load existing vouches for this newcomer
        let mut vouches: Vec<Address> = env
            .storage()
            .instance()
            .get(&DataKey::Vouches(newcomer.clone()))
            .unwrap_or(Vec::new(&env));

        // Prevent double vouching
        assert!(!vouches.contains(&voucher), "Already vouched for this address");

        // Record the vouch
        vouches.push_back(voucher.clone());
        env.storage()
            .instance()
            .set(&DataKey::Vouches(newcomer.clone()), &vouches);

        env.events().publish(
            (Symbol::new(&env, "vouched"),),
            (voucher, newcomer),
        );
    }

    /// Return how many vouches an address has received (read-only).
    pub fn get_vouches(env: Env, address: Address) -> u32 {
        let vouches: Vec<Address> = env
            .storage()
            .instance()
            .get(&DataKey::Vouches(address))
            .unwrap_or(Vec::new(&env));
        vouches.len()
    }
}

mod test;