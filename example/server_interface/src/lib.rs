use ffi_rpc::{
    abi_stable, async_trait, bincode,
    ffi_rpc_macro::{self, plugin_api},
};

#[plugin_api(Server)]
pub trait ServerApi {
    async fn add() -> i32;
}
