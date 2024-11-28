# ffi_rpc

[![crates.io](https://img.shields.io/crates/v/ffi_rpc?label=crates.io&style=flat-square)](https://crates.io/crates/ffi_rpc)
[![docs.rs](https://img.shields.io/docsrs/ffi_rpc?style=flat-square)](https://docs.rs/ffi_rpc/latest)
![license](https://img.shields.io/github/license/mxwxz/ffi_rpc?style=flat-square)

Use FFI with RPC! The ABI is stable, any serializable type can be safely transferred through the FFI boundary.

## Why this crate
It has been quite a long time that the Rust does not have a stable ABI. Developing a plugin system is a challenging work since we have meet several problems such as `Segmentation fault`, `Bus error` and unexpected behaviors across the FFI boundary with `libloading`. The bugs exist only at runtime, depending on a lot of variables (OS type, rustc version, etc.). It is a nightmare to debug these errors.

Luckily, we have `abi_stable` crate with can provide a working stable ABI for us. However, it would be tricky and complex to introduce customized types to the interface. Thus, we have this crate to transfer any serializable type across the FFI boundary.

## Limitations
1. Generic is not supported.
2. Panic on incompatible API/Library, please manage your API version by yourself.

## Quick start
Assume you have three projects:
- server: typically the binary which will load dynamic libraries and use the FFI functions.
- client(plugin): the `.dylib/.so/.dll` library that defines the interface.
- client_interface: the interface that will be shared between server and client.

### client_interface
1. Add `ffi_rpc` to `[dependencies]` in `Cargo.toml`.
2. In `lib.rs`:
    ```rust
    use ffi_rpc::{
        abi_stable, async_trait, bincode,
        ffi_rpc_macro::{self, plugin_api},
    };

    #[plugin_api(Client)]
    pub trait ClientApi {
        async fn add(a: i32, b: i32) -> i32;
    }
    ```
How to split one interface into multiple traits: [example](example/client1_interface/src/lib.rs).

### client
1. Add `abi_stable = "0.11"` and `ffi_rpc` to `[dependencies]` in `Cargo.toml`.
2. In `lib.rs`:
    ```rust
    use ffi_rpc::{
        abi_stable::prefix_type::PrefixTypeTrait,
        async_ffi, async_trait, bincode,
        ffi_rpc_macro::{plugin_impl_call, plugin_impl_instance, plugin_impl_root, plugin_impl_trait},
        registry::Registry,
    };

    #[plugin_impl_instance(|| Api{})]
    #[plugin_impl_root]
    #[plugin_impl_call(client_interface::ClientApi)]    // must use full path
    struct Api;

    #[plugin_impl_trait]
    impl client_interface::ClientApi for Api {   // must use full path
        async fn add(&self, _: &Registry, a: i32, b: i32) -> i32 {
            a + b
        }
    }
    ``` 
How to implement multiple interfaces: [example](example/client1/src/lib.rs).

How to invoke other clients: [example](example/client2/src/lib.rs).

### server
1. Init the registry:
    ```rust
    let mut r = Registry::default();
    ```
2. Init all clients:
    ```rust
    let lib = client_interface::Client::new(
        format!("./target/debug/{}client{}", DLL_PREFIX, DLL_SUFFIX).as_ref(),
        &mut r,
        "client",
    ).unwrap();
    ```
3. Invoke methods:
    ```rust
    let ret = lib.add(&r, &1, &2).await;
    ```
How to mock a client: [example](example/server/src/main.rs).

## Black magic
Customize `_ffi_call` to route to different implementations manually.
```rust
#[sabi_extern_fn]
pub fn _ffi_call(
    func: RString,      // function to call `Trait::Method`.
    reg: &Registry,     // registry.
    param: RVec<u8>,    // function params.
) -> BorrowingFfiFuture<'_, RVec<u8>> {
    BorrowingFfiFuture::new(async move {
        if func.as_str().starts_with("crate::mod::Trait1::") {
            return crate::mod::Trait1Impl::parse_crate_mod_Trait1(func, reg, param).await;
        }
        if func.as_str().starts_with("crate::mod::Trait2::") {
            return crate::mod::Trait2Impl::parse_crate_mod_Trait2(func, reg, param).await;
        }
        panic!("Function is not defined in the library");
    })
}
```
