use client1_interface::{Client1, Param};
use ffi_rpc::{
    abi_stable::prefix_type::PrefixTypeTrait,
    async_ffi, async_trait,
    ffi_rpc_macro::{plugin_impl_call, plugin_impl_instance, plugin_impl_root, plugin_impl_trait},
    registry::Registry,
    rmp_serde, tokio,
};
use server_interface::Server;

#[plugin_impl_instance(|| Api{})]
#[plugin_impl_root]
#[plugin_impl_call(client2_interface::Client2Api)]
struct Api;

#[plugin_impl_trait]
impl client2_interface::Client2Api for Api {
    async fn add(&self, r: &Registry, a: i32, b: i32) -> i32 {
        let _ = tokio::spawn(async {
            println!("Spawn a tokio task");
        })
        .await;
        let t = Client1::from(r.get("client1").unwrap())
            .add(r, &Param { a: 7, b: 8 }, &9)
            .await;
        let m = Server::from(r.get("server").unwrap()).add(r).await;
        let o = Client1::from(r.get("client1").unwrap())
            .minus(r, &100, &50)
            .await;
        a + b + t + m + o
    }
}
