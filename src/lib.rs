use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::{UnorderedMap, UnorderedSet};
use near_sdk::env::log_str;
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{env, near_bindgen, AccountId, PanicOnDefault, Promise};
use std::ops::Div;

#[near_bindgen]
#[derive(BorshSerialize, BorshDeserialize)]
struct Moderators {
    moderators: UnorderedSet<AccountId>,
}

#[near_bindgen]
impl Moderators {
    fn new() -> Self {
        Moderators {
            moderators: UnorderedSet::new(b"a"),
        }
    }

    fn add_moderator(&mut self, account_id: &AccountId) {
        self.moderators.insert(account_id);
    }

    fn is_moderator(&self, account_id: &AccountId) -> bool {
        self.moderators.contains(account_id)
    }
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct DailyRoutine {
    moderator_require_amount: u128,
    platform_fee: u128,
    /**
     * =================
     * Challenge info
     * =================
     */
    // challngeId => challengeInfo
    challenge_info: UnorderedMap<u128, ChallengeInfo>,
    // challngeId => participants
    participants: UnorderedMap<u128, Vec<AccountId>>,
    // challngeId => totalBettingAmount
    total_betting_amount: UnorderedMap<u128, u128>,
    /**
     * =================
     * User
     * =================
     */
    // challngeId => user => excuteCount
    execute_count: UnorderedMap<u128, UnorderedMap<AccountId, u128>>,
    // challngeId => user => index => verifiedCount
    // TODO(not test): verified_count: UnorderedMap<u128, UnorderedMap<AccountId, UnorderedMap<u128, u128>>>,
    // challngeId => user => bettingAmount
    betting_amount: UnorderedMap<u128, UnorderedMap<AccountId, u128>>,
    // hash to prevent duplicate verification
    // 0 => not used hash, 1 => used hash
    execute_hash: UnorderedMap<Vec<u8>, u128>,
    // address => challngeId[]
    participated_challenge_ids: UnorderedMap<AccountId, Vec<u128>>,
    /**
     * =================
     * Moderator
     * =================
     */
    // challngeId => moderator => verifyCount
    verify_count: UnorderedMap<u128, UnorderedMap<AccountId, u128>>,
    // challngeId => moderator => verifyAmount
    moderator_amount: UnorderedMap<AccountId, u128>,
    /**
     * =================
     * authority
     * =================
     */
    owner: AccountId,
    moderators: Moderators,
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, Deserialize, Serialize)]
pub struct ChallengeInfo {
    // Maximum amount that can be bet on challenge
    max_betting_price: u128,
    // Minimum amount that can be bet on challenge
    min_betting_price: u128,
    // Maximum number of participants
    max_participants: u128,
    // The loser will be given back (failersRetrieveRatio/100)% of the bet amount to the betters.
    failers_retrieve_ratio: u8,
    // Number of executes to succeed
    success_condition: u16,
    // Number of executes to draw
    draw_condition: u16,
    // start time
    start_time: u64,
    // end time
    end_time: u64,
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, Deserialize, Serialize)]
pub struct VerifyUnit {
    challenge_id: u128,
    user: AccountId,
    index: u128,
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, Deserialize, Serialize)]
pub struct CheckUnit {
    lost_amount: u128,
    winner_count: u128,
}

#[near_bindgen]
impl DailyRoutine {
    #[init]
    pub fn new(owner_id: AccountId) -> Self {
        Self {
            moderator_require_amount: 0,
            platform_fee: 0,
            challenge_info: UnorderedMap::new(b"b"),
            participants: UnorderedMap::new(b"c"),
            total_betting_amount: UnorderedMap::new(b"d"),
            execute_count: UnorderedMap::new(b"e"),
            // TODO(not test): verified_count: UnorderedMap::new(b"f"),
            betting_amount: UnorderedMap::new(b"g"),
            execute_hash: UnorderedMap::new(b"h"),
            participated_challenge_ids: UnorderedMap::new(b"i"),
            verify_count: UnorderedMap::new(b"j"),
            moderator_amount: UnorderedMap::new(b"k"),
            owner: owner_id,
            moderators: Moderators::new(),
        }
    }

    pub fn setting(&mut self, moderator_require_amount: u128, platform_fee: u128) {
        self.check_owner();
        self.moderator_require_amount = moderator_require_amount;
        self.platform_fee = platform_fee;
    }

    pub fn challenge_setting(&mut self, challenge_id: u128, challenge_info: ChallengeInfo) {
        self.check_owner();
        self.challenge_info.insert(&challenge_id, &challenge_info);
    }

    pub fn get_challenge_info(&self, challenge_id: u128) -> ChallengeInfo {
        self.challenge_info.get(&challenge_id).unwrap()
    }

    pub fn get_participants(&self, challenge_id: u128) -> Vec<AccountId> {
        self.participants.get(&challenge_id).unwrap_or_default()
    }

    pub fn get_platform_fee(&self) -> u128 {
        self.platform_fee.clone()
    }

    pub fn get_participated_challenge_ids(&self, account_id: &AccountId) -> Vec<u128> {
        self.participated_challenge_ids
            .get(&account_id.clone())
            .unwrap_or_default()
    }

    pub fn get_total_betting_amount(&self, challenge_id: u128) -> u128 {
        self.total_betting_amount
            .get(&challenge_id)
            .unwrap_or_default()
            .div(1_000_000_000_000_000_000_000_000)
    }

    #[payable]
    pub fn participate(&mut self, challenge_id: u128, value: u128) {
        assert!(
            self.participants
                .get(&challenge_id)
                .map(|participants| participants.len())
                < self
                    .challenge_info
                    .get(&challenge_id)
                    .map(|info| info.max_participants as usize),
            "participants are full"
        );

        // assert!(
        //     self.challenge_info
        //         .get(&challenge_id)
        //         .map(|info| info.start_time <= env::block_timestamp()
        //             && env::block_timestamp() <= info.end_time)
        //         .unwrap_or(false),
        //     "challenge is not available"
        // );

        assert!(
            self.betting_amount
                .get(&challenge_id)
                .map(|amount_map| amount_map
                    .get(&env::predecessor_account_id())
                    .map(|amount| amount == 0)
                    .unwrap_or(true))
                .unwrap_or(true),
            "Can't participate"
        );
        assert!(
            self.challenge_info
                .get(&challenge_id)
                .map(|info| info.min_betting_price <= value && value <= info.max_betting_price)
                .unwrap_or(false),
            "price is not proper"
        );

        let real_value: u128 = value * 1_000_000_000_000_000_000_000_000;

        Promise::new(self.owner.clone()).transfer(real_value.clone());

        let participants: Option<Vec<AccountId>> = self.participants.get(&challenge_id);

        if let Some(mut participants) = participants {
            participants.push(env::predecessor_account_id());
            self.participants.insert(&challenge_id, &participants);
        } else {
            let mut new_participants: Vec<AccountId> = Vec::new();
            new_participants.push(env::predecessor_account_id());
            self.participants.insert(&challenge_id, &new_participants);
        }
        //betting amount
        let betting_amount_map: Option<UnorderedMap<AccountId, u128>> =
            self.betting_amount.get(&challenge_id);
        if let Some(mut amount_map) = betting_amount_map {
            let betting_amount: Option<u128> = amount_map.get(&env::predecessor_account_id());
            if let Some(mut _amount) = betting_amount {
                _amount = real_value.clone();
            } else {
                amount_map.insert(&env::predecessor_account_id(), &real_value.clone());
            }
            self.betting_amount.insert(&challenge_id, &amount_map);
        } else {
            let prefix: [u8; 32] =
                near_sdk::env::sha256_array(format!("betting amount {}", challenge_id).as_bytes());
            let prefix_slice: &[u8] = &prefix[..];
            let mut new_amount_map = UnorderedMap::new(prefix_slice);
            new_amount_map.insert(&env::predecessor_account_id(), &real_value.clone());
            self.betting_amount.insert(&challenge_id, &new_amount_map);
        }
        //total betting amount
        let total_betting_amount: Option<u128> = self.total_betting_amount.get(&challenge_id);
        if let Some(mut _amount) = total_betting_amount {
            _amount += real_value.clone();
            self.total_betting_amount.insert(&challenge_id, &_amount);
        } else {
            self.total_betting_amount
                .insert(&challenge_id, &real_value.clone());
        }
        //participated challenge ids
        let participated_challenge_ids: Option<Vec<u128>> = self
            .participated_challenge_ids
            .get(&env::predecessor_account_id());
        if let Some(mut _ids) = participated_challenge_ids {
            _ids.push(challenge_id);
            self.participated_challenge_ids
                .insert(&env::predecessor_account_id(), &_ids);
        } else {
            let mut new_ids: Vec<u128> = Vec::new();
            new_ids.push(challenge_id);
            self.participated_challenge_ids
                .insert(&env::predecessor_account_id(), &new_ids);
        }
    }

    pub fn verify(&mut self, moderator: AccountId, verify_units: Vec<VerifyUnit>) {
        self.check_moderator(&moderator);
        for verify_unit in verify_units {
            let challenge_id: u128 = verify_unit.challenge_id;
            let user: AccountId = verify_unit.user;
            let index: u128 = verify_unit.index;

            // TODO(not test)
            // let calculated_hash = near_sdk::env::keccak256(
            //     format!("{}{}{}{}", moderator, challenge_id, user, index).as_bytes(),
            // );
            let calculated_hash: Vec<u8> =
                near_sdk::env::keccak256(format!("{}{}{}", challenge_id, user, index).as_bytes());

            assert!(
                self.execute_hash.get(&calculated_hash).unwrap_or(0) == 0,
                "already verified"
            );

            assert!(
                self.betting_amount
                    .get(&challenge_id)
                    .and_then(|m| m.get(&user))
                    .unwrap_or(0)
                    > 0,
                "invalid user"
            );

            // TODO(not test): verified_count => if over specific count, then execute_count++

            // execute_count
            let mut _execute_count: UnorderedMap<AccountId, u128> =
                self.execute_count.get(&challenge_id).unwrap_or_else(|| {
                    let prefix: [u8; 32] =
                        near_sdk::env::sha256_array(format!("excute{}", challenge_id).as_bytes());
                    let prefix_slice: &[u8] = &prefix[..];
                    let inner_map: UnorderedMap<AccountId, u128> = UnorderedMap::new(prefix_slice);
                    self.execute_count.insert(&challenge_id, &inner_map);
                    inner_map
                });

            let mut execute_count_: u128 = match _execute_count.get(&user) {
                Some(count) => count,
                None => 0,
            };
            execute_count_ += 1;
            _execute_count.insert(&user, &execute_count_);
            self.execute_count.insert(&challenge_id, &_execute_count);

            // verify_count
            let mut _verify_count: UnorderedMap<AccountId, u128> =
                self.verify_count.get(&challenge_id).unwrap_or_else(|| {
                    let prefix: [u8; 32] =
                        near_sdk::env::sha256_array(format!("verify{}", challenge_id).as_bytes());
                    let prefix_slice: &[u8] = &prefix[..];
                    let inner_map: UnorderedMap<AccountId, u128> = UnorderedMap::new(prefix_slice);
                    self.verify_count.insert(&challenge_id, &inner_map);
                    inner_map
                });
            let mut verify_count_: u128 = match _verify_count.get(&moderator) {
                Some(count) => count,
                None => 0,
            };
            verify_count_ += 1;
            _verify_count.insert(&moderator, &verify_count_);
            self.verify_count.insert(&challenge_id, &_verify_count);

            self.execute_hash.insert(&calculated_hash, &1);

            log_str(
                format!(
                    "Verified: challenge_id={}, user={}, index={}",
                    challenge_id,
                    user.clone(),
                    index
                )
                .as_str(),
            );
        }
    }

    #[payable]
    pub fn settle_winner(&mut self, challenge_id: u128) {
        self.check_owner();

        let mut spent_amount: u128 = 0;
        let mut lost_amount: u128 = 0;
        let total_amount: Option<u128> = self.total_betting_amount.get(&challenge_id);

        let challenge_info: &Option<ChallengeInfo> = &self.challenge_info.get(&challenge_id);
        let mut winners: Vec<Option<AccountId>> = vec![None; 100];

        let participants: Option<Vec<AccountId>> = self.participants.get(&challenge_id).clone();

        for participant in participants.unwrap_or_default().iter() {
            let betting_amount: u128 = self
                .betting_amount
                .get(&challenge_id)
                .and_then(|map: UnorderedMap<AccountId, u128>| map.get(participant))
                .unwrap_or_default();
            let execute_count = self
                .execute_count
                .get(&challenge_id)
                .and_then(|map: UnorderedMap<AccountId, u128>| map.get(participant))
                .unwrap_or_default();
            if execute_count >= challenge_info.as_ref().unwrap().success_condition as u128 {
                winners.push(Some(participant.clone()));
            } else if execute_count >= challenge_info.as_ref().unwrap().draw_condition as u128 {
                Promise::new(participant.clone()).transfer(betting_amount);
                spent_amount += betting_amount;
            } else {
                Promise::new(participant.clone()).transfer(
                    (betting_amount
                        * challenge_info.as_ref().unwrap().failers_retrieve_ratio as u128)
                        / 100,
                );
                spent_amount += (betting_amount
                    * challenge_info.as_ref().unwrap().failers_retrieve_ratio as u128)
                    / 100;
                lost_amount += (betting_amount
                    * (100 - challenge_info.as_ref().unwrap().failers_retrieve_ratio as u128))
                    / 100;
            }
        }

        for winner in winners.iter() {
            if let Some(account_id) = winner {
                let betting_amount: u128 = self
                    .betting_amount
                    .get(&challenge_id)
                    .and_then(|map: UnorderedMap<AccountId, u128>| map.get(account_id))
                    .unwrap_or_default();
                // Winner takes not only the bet amount but also the loser amount as much as the bet rate
                Promise::new(account_id.clone()).transfer(
                    betting_amount + (lost_amount * betting_amount) / total_amount.unwrap(),
                );
                spent_amount +=
                    betting_amount + (lost_amount * betting_amount) / total_amount.unwrap();
            }
        }

        let moderators: Vec<AccountId> = self.get_moderators();
        let verify_count_sum: u128 = moderators
            .iter()
            .map(|m: &AccountId| {
                self.verify_count
                    .get(&challenge_id)
                    .unwrap()
                    .get(m)
                    .unwrap()
            })
            .sum();

        for moderator in moderators.iter() {
            let transfer_amount: u128 = ((total_amount.unwrap() - spent_amount)
                * self
                    .verify_count
                    .get(&challenge_id)
                    .unwrap()
                    .get(moderator)
                    .unwrap()
                * (100 - self.platform_fee))
                / (verify_count_sum * 100);
            Promise::new(moderator.clone()).transfer(transfer_amount);
        }

        log_str(format!("Settled: challenge_id={}", challenge_id).as_str());
    }

    /**
     * ===========
     * Authority
     * ===========
     */
    fn check_owner(&self) {
        if !self.is_owner(env::predecessor_account_id()) {
            env::panic_str("Caller is not the owner.");
        }
    }

    pub fn add_moderator(&mut self, moderator: AccountId) {
        self.check_owner();
        self.moderators.add_moderator(&moderator);
    }
    
    fn check_moderator(&self, account_id: &AccountId) {
        if !self.moderators.is_moderator(account_id) {
            env::panic_str("Caller is not a moderator.");
        }
    }

    fn get_moderators(&self) -> Vec<AccountId> {
        self.moderators.moderators.iter().collect()
    }

    pub fn get_owner(&self) -> AccountId {
        self.owner.clone()
    }

    pub fn is_owner(&self, account_id: AccountId) -> bool {
        account_id == self.owner
    }

    pub fn transfer_ownership(&mut self, new_owner: AccountId) {
        assert!(
            env::predecessor_account_id() == self.owner,
            "Only the owner can transfer ownership"
        );
        self.owner = new_owner;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::env;
    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::testing_env;

    // Sets up a basic context for testing.
    fn basic_context() -> VMContextBuilder {
        let mut builder: VMContextBuilder = VMContextBuilder::new();
        builder.signer_account_id(accounts(0));
        builder.current_account_id(accounts(0));
        builder.predecessor_account_id(accounts(0));
        builder.account_balance(10 * 10u128.pow(26));
        builder
    }

    fn bob_context() -> VMContextBuilder {
        let mut builder: VMContextBuilder = VMContextBuilder::new();
        builder.signer_account_id(accounts(1));
        builder.current_account_id(accounts(1));
        builder.predecessor_account_id(accounts(1));
        builder.account_balance(10 * 10u128.pow(26));
        builder
    }

    #[test]
    fn test_participate() {
        let owner_context: near_sdk::VMContext = basic_context().build();
        testing_env!(owner_context.clone());

        let mut contract: DailyRoutine = DailyRoutine::new(accounts(0));
        contract.setting(100, 10);

        let challenge_id: u128 = 1;
        let current_timestamp: u64 = env::block_timestamp();
        let challenge_info: ChallengeInfo = ChallengeInfo {
            max_betting_price: 1000,
            min_betting_price: 10,
            max_participants: 10,
            failers_retrieve_ratio: 50,
            success_condition: 5,
            draw_condition: 3,
            start_time: current_timestamp,
            end_time: current_timestamp + 1000,
        };
        contract.challenge_setting(challenge_id, challenge_info);

        let value: u128 = 100;

        let bob_context: near_sdk::VMContext = bob_context().build();

        testing_env!(bob_context);

        // Call the `participate` function
        contract.participate(challenge_id, value);

        testing_env!(owner_context.clone());

        contract.participate(challenge_id, value * 2);

        // Assert the changes in contract state
        let participants: Vec<AccountId> =
            contract.participants.get(&challenge_id).unwrap_or_default();
        assert_eq!(participants.len(), 2);

        assert_eq!(participants[0], accounts(1));

        let betting_amount: UnorderedMap<AccountId, u128> =
            contract.betting_amount.get(&challenge_id).unwrap();
        assert_eq!(
            betting_amount.get(&accounts(1)).clone().unwrap(),
            value * 1_000_000_000_000_000_000_000_000,
            "betting amount not updated"
        );
        //check total betting amount
        let total_betting_amount: u128 = contract
            .total_betting_amount
            .get(&challenge_id)
            .unwrap()
            .div(1_000_000_000_000_000_000_000_000);
        assert_eq!(
            total_betting_amount,
            value * 3,
            "total betting amount not updated"
        );
    }

    #[test]
    fn test_verify() {
        let owner_context: near_sdk::VMContext = basic_context().build();
        testing_env!(owner_context.clone());

        let mut contract: DailyRoutine = DailyRoutine::new(accounts(0));
        contract.setting(100, 10);

        let challenge_id: u128 = 1;
        let current_timestamp: u64 = env::block_timestamp();
        let challenge_info: ChallengeInfo = ChallengeInfo {
            max_betting_price: 1000,
            min_betting_price: 10,
            max_participants: 10,
            failers_retrieve_ratio: 50,
            success_condition: 5,
            draw_condition: 3,
            start_time: current_timestamp,
            end_time: current_timestamp + 1000,
        };
        contract.challenge_setting(challenge_id, challenge_info);
        let moderator: AccountId = accounts(0);
        let user: AccountId = accounts(1);
        let index: u128 = 0;

        // Add moderator
        contract.add_moderator(moderator.clone());

        let bob_context: near_sdk::VMContext = bob_context().build();

        testing_env!(bob_context);
        // Participate in the challenge
        contract.participate(challenge_id, 100);

        // Verify the participant
        let verify_units: Vec<VerifyUnit> = vec![VerifyUnit {
            challenge_id,
            user: user.clone(),
            index,
        }];

        testing_env!(owner_context.clone());
        contract.verify(moderator.clone(), verify_units);

        // Assert the changes in contract state
        let execute_count = contract
            .execute_count
            .get(&challenge_id)
            .expect("Error: Challenge ID not found")
            .get(&user)
            .expect("Error: User not found");
        assert_eq!(execute_count, 1, "execute count not updated");

        let verify_count = contract
            .verify_count
            .get(&challenge_id)
            .expect("Error: Challenge ID not found")
            .get(&moderator)
            .expect("Error: Moderator not found");
        assert_eq!(verify_count, 1, "verify count not updated");
    }

    #[test]
    fn test_settle_winner() {
        let owner_context = basic_context().build();
        testing_env!(owner_context.clone());

        let mut contract = DailyRoutine::new(accounts(0));
        let challenge_id = 1;
        let current_timestamp = env::block_timestamp();
        let challenge_info = ChallengeInfo {
            max_betting_price: 1000,
            min_betting_price: 10,
            max_participants: 10,
            failers_retrieve_ratio: 50,
            success_condition: 5,
            draw_condition: 3,
            start_time: current_timestamp,
            end_time: current_timestamp + 1,
        };
        contract.challenge_setting(challenge_id, challenge_info);

        let user = accounts(1);

        let bob_context = bob_context().build();

        testing_env!(bob_context.clone());

        let balance = env::account_balance();
        log_str(format!("balance: {}", balance).as_str());

        // Participate in the challenge
        contract.participate(challenge_id, 100);
        let moderator = accounts(0);
        let index = 0;
        let balance = env::account_balance();
        log_str(format!("balance: {}", balance).as_str());
        // Add moderator

        testing_env!(owner_context.clone());
        contract.add_moderator(moderator.clone());
        // Verify the participant
        let verify_units = vec![VerifyUnit {
            challenge_id,
            user: user.clone(),
            index,
        }];

        contract.verify(moderator.clone(), verify_units);
        // Settle the winner
        contract.settle_winner(challenge_id);

        // Assert the changes in contract state
        let execute_count = contract
            .execute_count
            .get(&challenge_id)
            .unwrap()
            .get(&user)
            .unwrap();
        assert_eq!(execute_count, 1, "execute count not updated");

        //balance check
        let balance = env::account_balance();
        log_str(format!("balance: {}", balance).as_str());

        testing_env!(bob_context.clone());
        let balance = env::account_balance();
        log_str(format!("balance: {}", balance).as_str());
    }
}
