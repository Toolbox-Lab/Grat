pub mod disk;
pub mod provider;
pub mod store;
pub mod wasm;

pub fn set_bypass(enabled: bool) {
    store::set_bypass(enabled);
}
