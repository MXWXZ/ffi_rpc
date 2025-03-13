use ffi_rpc::{
    abi_stable, async_trait,
    ffi_rpc_macro::{self, plugin_api},
    rmp_serde,
};

#[plugin_api(Client2)]
pub trait Client2Api {
    async fn add(a: i32, b: i32) -> i32;
}
