//! Vulnerable example for `no-archival-handling`: `balance_of` unwraps the
//! `.get()` result directly, so any address with no entry (never funded,
//! or its entry already archived) panics the contract instead of reading
//! as zero.
#![no_std]

use soroban_sdk::{contract, contractimpl, Address, Env};

#[contract]
pub struct Contract;

#[contractimpl]
impl Contract {
    pub fn deposit(env: Env, user: Address, amount: i128) {
        env.storage().persistent().set(&user, &amount);
        env.storage()
            .persistent()
            .extend_ttl(&user, 100_000, 535_680);
    }

    pub fn balance_of(env: Env, user: Address) -> i128 {
        env.storage().persistent().get(&user).unwrap()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::testutils::Address as _;

    #[test]
    fn deposit_and_read_balance() {
        let env = Env::default();
        let contract_id = env.register(Contract, ());
        let client = ContractClient::new(&env, &contract_id);
        let user = Address::generate(&env);

        client.deposit(&user, &100);
        assert_eq!(client.balance_of(&user), 100);
    }

    #[test]
    #[should_panic]
    fn reading_an_unfunded_address_panics() {
        let env = Env::default();
        let contract_id = env.register(Contract, ());
        let client = ContractClient::new(&env, &contract_id);
        let stranger = Address::generate(&env);

        // This demonstrates the actual bug: no entry was ever written for
        // `stranger`, so `.get(..).unwrap()` panics instead of returning 0.
        client.balance_of(&stranger);
    }
}
