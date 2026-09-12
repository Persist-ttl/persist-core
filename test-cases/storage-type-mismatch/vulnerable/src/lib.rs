//! Vulnerable example for `storage-type-mismatch`: the admin address is
//! contract-wide config data, but it's stored in Persistent storage
//! (per-entry TTL bookkeeping) instead of Instance storage (TTL tied to the
//! contract instance itself, which is what admin/config data should ride
//! along with).
#![no_std]

use soroban_sdk::{contract, contractimpl, symbol_short, Address, Env};

#[contract]
pub struct Contract;

#[contractimpl]
impl Contract {
    pub fn set_admin(env: Env, admin: Address) {
        env.storage().persistent().set(&symbol_short!("admin"), &admin);
    }

    pub fn get_admin(env: Env) -> Option<Address> {
        env.storage().persistent().get(&symbol_short!("admin"))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::testutils::Address as _;

    #[test]
    fn set_and_read_admin() {
        let env = Env::default();
        let contract_id = env.register(Contract, ());
        let client = ContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);

        client.set_admin(&admin);
        assert_eq!(client.get_admin(), Some(admin));
    }
}
