//! Vulnerable example for `temporary-storage-misuse`: `get_session` reads a
//! key from Temporary storage that nothing in this contract ever writes
//! back, so once it expires the "session" data is gone for good (Temporary
//! entries are deleted, not archived) rather than merely stale.
#![no_std]

use soroban_sdk::{contract, contractimpl, Env, Symbol};

#[contract]
pub struct Contract;

#[contractimpl]
impl Contract {
    pub fn get_session(env: Env, id: Symbol) -> u32 {
        env.storage().temporary().get(&id).unwrap_or(0)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::symbol_short;

    #[test]
    fn reads_missing_session_as_default() {
        let env = Env::default();
        let contract_id = env.register(Contract, ());
        let client = ContractClient::new(&env, &contract_id);

        assert_eq!(client.get_session(&symbol_short!("sess1")), 0);
    }
}
