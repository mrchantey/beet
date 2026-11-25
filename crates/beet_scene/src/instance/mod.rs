//! Instantiating one document in another.
mod instance;
mod prefab_cache;

pub use instance::Activation;
pub use instance::InstanceTemplate;
pub(crate) use instance::activate_on_insert;
pub use prefab_cache::InstancePrefab;
pub use prefab_cache::PrefabCache;
pub(crate) use prefab_cache::forget_changed_prefabs;
