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


pub struct BevyRsxPlugin {
	pub app: Box<dyn 'static + Send + Sync + Fn() -> WebNode>,
}

impl BevyRsxPlugin {
	pub fn new(app: impl 'static + Send + Sync + Fn() -> WebNode) -> Self {
		Self { app: Box::new(app) }
	}
}

impl Plugin for BevyRsxPlugin {
	fn build(&self, app: &mut App) {
		BevyRuntime::with_mut(|rt_app| std::mem::swap(rt_app, app));
		let _entities = RsxToBevy::spawn((self.app)()).unwrap();
		BevyRuntime::with_mut(|rt_app| std::mem::swap(rt_app, app));
	}
}
