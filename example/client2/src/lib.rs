use client1_interface::{Client1, Client1Api1, Client1Api2, Param};
use client2_interface::Client2Api;
use ffi_rpc::{
    abi_stable::prefix_type::PrefixTypeTrait,
    async_ffi, async_trait, bincode,
    ffi_rpc_macro::{plugin_impl_call, plugin_impl_instance, plugin_impl_root, plugin_impl_trait},
    registry::Registry,
};
use server_interface::{Server, ServerApi};

#[plugin_impl_instance(|| Api{})]
#[plugin_impl_root]
#[plugin_impl_call(Client2Api)]
struct Api;

#[plugin_impl_trait]
impl Client2Api for Api {
    async fn add(&self, r: &Registry, a: i32, b: i32) -> i32 {
        let t = Client1::from(r.get("client1"))
            .add(r, Param { a: 7, b: 8 }, 9)
            .await;
        let m = Server::from(r.get("server")).add(r).await;
        let o = Client1::from(r.get("client1")).minus(r, 100, 50).await;
        a + b + t + m + o
    }
}
