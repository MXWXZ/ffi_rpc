//! Use FFI with RPC! The ABI is stable, any serializable type can be safely transferred through the FFI boundary.
//!
//! Please refer to our [crate.io](https://crates.io/crates/ffi_rpc) and [Github](https://github.com/MXWXZ/ffi_rpc) for more documents.
pub mod plugin;
pub mod registry;

pub use abi_stable;
pub use async_ffi;
pub use async_trait;
pub use bincode;
pub use ffi_rpc_macro;
