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
    Proposals,
    NextProposalId,
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

#[contracttype]
#[derive(Clone)]
pub enum ProposalType {
    ChangeAmount(i128),
    ChangeCycleLength(u64),
    AddMember(Address),
    RemoveMember(Address),
}

#[contracttype]
#[derive(Clone)]
pub struct Proposal {
    pub id: u32,
    pub proposer: Address,
    pub proposal_type: ProposalType,
    pub votes_yes: u32,
    pub votes_no: u32,
    pub voters: Vec<Address>,
    pub executed: bool,
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
        // `from` requires &Address, `to` accepts Address (MuxedAddress)
        let token_client = token::Client::new(&env, &circle.usdc_token);
        token_client.transfer(
            &member,
            env.current_contract_address(),
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
        // `from` requires &Address, `to` accepts Address (MuxedAddress)
        let recipient = circle.members.get(circle.payout_index).unwrap();
        let token_client = token::Client::new(&env, &circle.usdc_token);
        token_client.transfer(
            &env.current_contract_address(),
            recipient.clone(),
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

        let rep: u32 = env
            .storage()
            .instance()
            .get(&DataKey::Reputation(voucher.clone()))
            .unwrap_or(0);
        assert!(rep >= 50, "Need reputation >= 50 to vouch for others");

        let mut vouches: Vec<Address> = env
            .storage()
            .instance()
            .get(&DataKey::Vouches(newcomer.clone()))
            .unwrap_or(Vec::new(&env));

        assert!(!vouches.contains(&voucher), "Already vouched for this address");

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

    /// Any circle member can submit a proposal to change a circle rule.
    /// Returns the new proposal ID.
    pub fn propose(env: Env, proposer: Address, proposal_type: ProposalType) -> u32 {
        proposer.require_auth();

        let circle: Circle = env.storage().instance().get(&DataKey::Circle).unwrap();
        assert!(circle.members.contains(&proposer), "Only members can propose");

        let id: u32 = env
            .storage()
            .instance()
            .get(&DataKey::NextProposalId)
            .unwrap_or(0);

        let proposal = Proposal {
            id,
            proposer: proposer.clone(),
            proposal_type,
            votes_yes: 0,
            votes_no: 0,
            voters: Vec::new(&env),
            executed: false,
        };

        let mut proposals: Map<u32, Proposal> = env
            .storage()
            .instance()
            .get(&DataKey::Proposals)
            .unwrap_or(Map::new(&env));

        proposals.set(id, proposal);
        env.storage()
            .instance()
            .set(&DataKey::Proposals, &proposals);
        env.storage()
            .instance()
            .set(&DataKey::NextProposalId, &(id + 1));

        env.events().publish(
            (Symbol::new(&env, "proposed"),),
            (proposer, id),
        );

        id
    }

    /// A circle member votes yes or no on an open proposal.
    /// Each member gets exactly one vote per proposal.
    pub fn vote(env: Env, voter: Address, proposal_id: u32, vote_yes: bool) {
        voter.require_auth();

        let circle: Circle = env.storage().instance().get(&DataKey::Circle).unwrap();
        assert!(circle.members.contains(&voter), "Only members can vote");

        let mut proposals: Map<u32, Proposal> = env
            .storage()
            .instance()
            .get(&DataKey::Proposals)
            .unwrap();

        let mut proposal = proposals.get(proposal_id).unwrap();
        assert!(!proposal.executed, "Proposal already executed");
        assert!(
            !proposal.voters.contains(&voter),
            "Already voted on this proposal"
        );

        if vote_yes {
            proposal.votes_yes += 1;
        } else {
            proposal.votes_no += 1;
        }
        proposal.voters.push_back(voter.clone());

        proposals.set(proposal_id, proposal);
        env.storage()
            .instance()
            .set(&DataKey::Proposals, &proposals);

        env.events().publish(
            (Symbol::new(&env, "voted"),),
            (voter, proposal_id, vote_yes),
        );
    }

    /// Execute a passed proposal. Requires votes_yes > total_members / 2.
    /// Applies the rule change to the circle immediately.
    pub fn execute_proposal(env: Env, caller: Address, proposal_id: u32) {
        caller.require_auth();

        let mut circle: Circle = env.storage().instance().get(&DataKey::Circle).unwrap();

        let mut proposals: Map<u32, Proposal> = env
            .storage()
            .instance()
            .get(&DataKey::Proposals)
            .unwrap();

        let mut proposal = proposals.get(proposal_id).unwrap();
        assert!(!proposal.executed, "Proposal already executed");

        let total_members = circle.members.len();
        assert!(
            proposal.votes_yes > total_members / 2,
            "Proposal has not reached majority"
        );

        // Apply the rule change
        match proposal.proposal_type.clone() {
            ProposalType::ChangeAmount(new_amount) => {
                circle.contribution_amount = new_amount;
            }
            ProposalType::ChangeCycleLength(new_length) => {
                circle.cycle_length_secs = new_length;
            }
            ProposalType::AddMember(new_member) => {
                circle.members.push_back(new_member);
            }
            ProposalType::RemoveMember(member) => {
                let mut new_members = Vec::new(&env);
                for m in circle.members.iter() {
                    if m != member {
                        new_members.push_back(m);
                    }
                }
                circle.members = new_members;
            }
        }

        proposal.executed = true;
        proposals.set(proposal_id, proposal);

        env.storage().instance().set(&DataKey::Circle, &circle);
        env.storage()
            .instance()
            .set(&DataKey::Proposals, &proposals);

        env.events().publish(
            (Symbol::new(&env, "executed"),),
            proposal_id,
        );
    }

    /// Read a single proposal by ID (read-only).
    pub fn get_proposal(env: Env, proposal_id: u32) -> Proposal {
        let proposals: Map<u32, Proposal> = env
            .storage()
            .instance()
            .get(&DataKey::Proposals)
            .unwrap();
        proposals.get(proposal_id).unwrap()
    }
}

mod test;