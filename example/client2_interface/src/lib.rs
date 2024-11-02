use ffi_rpc::{
    abi_stable, async_trait, bincode,
    ffi_rpc_macro::{self, plugin_api},
};

#[plugin_api(Client2)]
pub trait Client2Api {
    async fn add(a: i32, b: i32) -> i32;
}
