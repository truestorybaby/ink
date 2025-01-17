#![cfg_attr(not(feature = "std"), no_std)]

/// Re-export the `ContractRef` generated by the ink! codegen.
///
/// This let's other crates which pull this contract in as a dependency to interact with this
/// contract in a type-safe way.
pub use self::other_contract::OtherContractRef;

#[ink::contract]
mod other_contract {

    #[ink(storage)]
    pub struct OtherContract {
        value: bool,
    }

    impl OtherContract {
        #[ink(constructor)]
        pub fn new(init_value: bool) -> Self {
            Self { value: init_value }
        }

        #[ink(message)]
        pub fn flip(&mut self) {
            self.value = !self.value;
        }

        #[ink(message)]
        pub fn get(&self) -> bool {
            self.value
        }
    }
}
