//! Fixed example for `storage-type-mismatch`: the admin address now lives
//! in Instance storage, tying its TTL to the contract instance rather than
//! managing it as a separate per-entry Persistent TTL.
#![no_std]

use soroban_sdk::{contract, contractimpl, symbol_short, Address, Env};

#[contract]
pub struct Contract;

#[contractimpl]
impl Contract {
    pub fn set_admin(env: Env, admin: Address) {
        env.storage()
            .instance()
            .set(&symbol_short!("admin"), &admin);
    }

    pub fn get_admin(env: Env) -> Option<Address> {
        env.storage().instance().get(&symbol_short!("admin"))
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
