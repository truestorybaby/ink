#[ink::storage_item(derive = false)]
#[ink::storage_item(derive = true)]
#[derive(Default)]
struct Contract<KEY: StorageKey = ManualKey<123>> {
    a: u16,
    b: u64,
    c: u128,
}

fn main() {}
