use ffi_rpc::{
    abi_stable, async_trait, bincode,
    ffi_rpc_macro::{plugin_api_struct, plugin_api_trait},
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Param {
    pub a: i32,
    pub b: i32,
}

#[plugin_api_struct]
pub struct Client1;

#[plugin_api_trait(Client1)]
pub trait Client1Api1 {
    async fn add(p: Param, offset: i32) -> i32;
}

#[plugin_api_trait(Client1)]
pub trait Client1Api2 {
    async fn minus(a: i32, b: i32) -> i32;
}
