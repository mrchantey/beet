#[cfg(feature = "bevy_default")]
mod bevy_event_registry;
mod bevy_runtime;
mod bevy_signal;
mod reflect_utils;
mod rsx_to_bevy;
#[cfg(feature = "bevy_default")]
pub use bevy_event_registry::*;
pub use bevy_runtime::*;
pub use bevy_signal::*;
pub use bevy_tree_idx::*;
pub use reflect_utils::*;
pub use rsx_to_bevy::*;
mod bevy_tree_idx;
// local
use crate::prelude::*;
use bevy::prelude::*;

#[extend::ext(name=MyTypeExt)]
pub impl App {
	fn spawn_rsx(&mut self, root: impl Fn() -> RsxRoot) -> &mut Self {
		BevyRuntime::with_mut(|rt_app| std::mem::swap(rt_app, self));
		let _entities = RsxToBevy::spawn(root()).unwrap();
		BevyRuntime::with_mut(|rt_app| std::mem::swap(rt_app, self));
		self
	}
}
