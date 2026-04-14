use crate::prelude::*;
use beet_core::prelude::*;

/// Plugin that registers all beet_net types for scene serialization.
///
/// Includes [`BucketPlugin`] for typed bucket and blob registration.
#[derive(Default)]
pub struct NetPlugin;

impl Plugin for NetPlugin {
	fn build(&self, app: &mut App) { app.init_plugin::<BucketPlugin>(); }
}
