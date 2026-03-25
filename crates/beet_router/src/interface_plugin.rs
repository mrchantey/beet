use crate::prelude::*;
use beet_core::prelude::*;

/// Plugin that registers the [`single_current_scene`] safety-net observer.
#[derive(Default)]
pub struct InterfacePlugin;

impl Plugin for InterfacePlugin {
	fn build(&self, app: &mut App) { app.add_observer(single_current_scene); }
}
