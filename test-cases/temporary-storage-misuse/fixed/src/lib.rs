//! Fixed example for `temporary-storage-misuse`: `touch_session` writes the
//! same key `get_session` reads, so the read path's assumption that the
//! value can be (re)written is actually backed by code.
#![no_std]

use soroban_sdk::{contract, contractimpl, Env, Symbol};

#[contract]
pub struct Contract;

#[contractimpl]
impl Contract {
    pub fn touch_session(env: Env, id: Symbol, value: u32) {
        env.storage().temporary().set(&id, &value);
    }

    pub fn get_session(env: Env, id: Symbol) -> u32 {
        env.storage().temporary().get(&id).unwrap_or(0)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::symbol_short;

    #[test]
    fn touch_then_read_session() {
        let env = Env::default();
        let contract_id = env.register(Contract, ());
        let client = ContractClient::new(&env, &contract_id);

        client.touch_session(&symbol_short!("sess1"), &42);
        assert_eq!(client.get_session(&symbol_short!("sess1")), 42);
    }
}
