use client1_interface::Param;
use ffi_rpc::{
    abi_stable::prefix_type::PrefixTypeTrait,
    async_ffi, async_trait,
    ffi_rpc_macro::{plugin_impl_call, plugin_impl_instance, plugin_impl_root, plugin_impl_trait},
    registry::Registry,
    rmp_serde, tokio,
};

#[plugin_impl_instance(|| Api(1))]
#[plugin_impl_root]
#[plugin_impl_call(client1_interface::Client1Api1, client1_interface::Client1Api2)]
struct Api(i32);

#[plugin_impl_trait]
impl client1_interface::Client1Api1 for Api {
    async fn add(&self, _: &Registry, p: Param, offset: i32) -> i32 {
        self.0 + p.a + p.b + offset
    }
}

#[plugin_impl_trait(&*API_INSTANCE)] // #[plugin_impl_trait] is also ok!
impl client1_interface::Client1Api2 for Api {
    async fn minus(&self, _: &Registry, a: i32, b: i32) -> i32 {
        a - b
    }
}
