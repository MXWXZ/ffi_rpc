use ffi_rpc::{
    abi_stable, async_trait,
    ffi_rpc_macro::{self, plugin_api},
    rmp_serde,
};

#[plugin_api(Server)]
pub trait ServerApi {
    async fn add() -> i32;
}
