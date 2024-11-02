use std::env::consts::{DLL_PREFIX, DLL_SUFFIX};

use client1_interface::{Client1, Client1Api1, Param};
use client2_interface::{Client2, Client2Api};
use ffi_rpc::{
    async_ffi, async_trait, bincode,
    ffi_rpc_macro::{plugin_impl_call, plugin_impl_instance, plugin_impl_mock, plugin_impl_trait},
    registry::Registry,
};
use server_interface::ServerApi;

#[plugin_impl_instance(||Server{})]
#[plugin_impl_call(ServerApi)]
#[plugin_impl_mock]
struct Server;

#[plugin_impl_trait]
impl ServerApi for Server {
    async fn add(&self, _: &ffi_rpc::registry::Registry) -> i32 {
        10
    }
}

#[tokio::main]
async fn main() {
    let mut r = Registry::default();
    Server::register_mock(&mut r, "server");

    let lib1 = Client1::new(
        format!("./target/debug/{}client1{}", DLL_PREFIX, DLL_SUFFIX).as_ref(),
        &mut r,
        "client1",
    )
    .unwrap();
    let ret = lib1.add(&r, Param { a: 2, b: 3 }, 4).await;
    println!("1+2+3+4 should be: {ret}");

    let lib2 = Client2::new(
        format!("./target/debug/{}client2{}", DLL_PREFIX, DLL_SUFFIX).as_ref(),
        &mut r,
        "client2",
    )
    .unwrap();
    let ret = lib2.add(&r, 1, 2).await;
    println!("1+2+1+7+8+9+10+100-50 should be: {ret}");
}
