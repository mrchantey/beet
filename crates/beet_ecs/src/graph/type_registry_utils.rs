use crate::prelude::*;
use bevy::prelude::*;

pub fn merge_type_registries(src: &AppTypeRegistry, dst: &mut AppTypeRegistry) {
	let src = src.read();
	let mut dst = dst.write();
	for registration in src.iter() {
		dst.add_registration(registration.clone());
	}
}

/// Register all types in [`T`] as well as those that get appended
/// to the graph by [`EntityGraph::spawn_with_options`]
/// with the exception of [[`TargetAgent`]] which gets reattached via [`DynGraph::spawn`]
pub fn append_beet_type_registry(registry: &AppTypeRegistry) {
	let mut registry = registry.write();
	registry.register::<NodeName>();
	registry.register::<Name>();
	registry.register::<Edges>();
	registry.register::<Running>();
	registry.register::<RunTimer>();
	registry.register::<BehaviorGraphRoot>();
}

pub fn append_beet_type_registry_with_generics<T: ActionTypes>(
	registry: &AppTypeRegistry,
) {
	append_beet_type_registry(registry);
	let mut registry = registry.write();
	T::register_types(&mut registry);
}
