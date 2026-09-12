//! Vulnerable example for `insufficient-ttl-margin`: the `extend_ttl` call
//! extends the entry by only 50 ledgers (a few minutes), so it has to be
//! bumped on almost every call or it archives.
#![no_std]

use soroban_sdk::{contract, contractimpl, Address, Env};

#[contract]
pub struct Contract;

#[contractimpl]
impl Contract {
    pub fn deposit(env: Env, user: Address, amount: i128) {
        env.storage().persistent().set(&user, &amount);
        env.storage().persistent().extend_ttl(&user, 10, 50);
    }

    pub fn balance_of(env: Env, user: Address) -> i128 {
        env.storage().persistent().get(&user).unwrap_or(0)
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
}
