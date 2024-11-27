use std::path::Path;

use abi_stable::{
    library::{lib_header_from_path, LibraryError, RootModule},
    package_version_strings,
    sabi_types::VersionStrings,
    std_types::{RString, RVec},
    StableAbi,
};
use async_ffi::BorrowingFfiFuture;

use crate::registry::Registry;

#[repr(C)]
#[derive(StableAbi)]
#[sabi(kind(Prefix(prefix_ref = PluginApiRef)))]
#[sabi(missing_field(panic))]
pub struct PluginApi {
    #[sabi(last_prefix_field)]
    pub call: for<'fut> extern "C" fn(
        RString,
        &'fut Registry,
        RVec<u8>,
    ) -> BorrowingFfiFuture<'fut, RVec<u8>>,
}

/// The RootModule trait defines how to load the root module of a library.
impl RootModule for PluginApiRef {
    abi_stable::declare_root_module_statics! {PluginApiRef}

    const BASE_NAME: &'static str = "plugin";
    const NAME: &'static str = "plugin";
    const VERSION_STRINGS: VersionStrings = package_version_strings!();
}

pub fn load_plugin(path: &Path) -> Result<PluginApiRef, LibraryError> {
    lib_header_from_path(path).and_then(|x| x.init_root_module::<PluginApiRef>())
}
