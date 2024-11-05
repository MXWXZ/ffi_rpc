use client1_interface::{Client1Api1, Client1Api2, Param};
use ffi_rpc::{
    abi_stable::prefix_type::PrefixTypeTrait,
    async_ffi, async_trait, bincode,
    ffi_rpc_macro::{plugin_impl_call, plugin_impl_instance, plugin_impl_root, plugin_impl_trait},
    registry::Registry,
};

#[plugin_impl_instance(|| Api(1))]
#[plugin_impl_root]
#[plugin_impl_call(Client1Api1, Client1Api2)]
struct Api(i32);

#[plugin_impl_trait]
impl Client1Api1 for Api {
    async fn add(&self, _: &Registry, p: &mut Param, offset: i32) -> i32 {
        self.0 + p.a + p.b + offset
    }
}

#[plugin_impl_trait]
impl Client1Api2 for Api {
    async fn minus(&self, _: &Registry, a: i32, b: i32) -> i32 {
        a - b
    }
}
