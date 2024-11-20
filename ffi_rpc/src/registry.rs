use abi_stable::{
    std_types::{RHashMap, RString},
    StableAbi,
};

use crate::plugin::PluginApiRef;

#[repr(C)]
#[derive(StableAbi, Default, Clone)]
pub struct Registry {
    pub item: RHashMap<RString, PluginApiRef>,
}

impl Registry {
    pub fn get(&self, id: &str) -> Option<PluginApiRef> {
        self.item.get(id).cloned()
    }
}
