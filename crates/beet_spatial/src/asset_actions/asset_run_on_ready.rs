use beet_flow::prelude::*;
// use beet_flow::prelude::*;
use beetmash::prelude::*;
use bevy::prelude::*;


/// A helper bundle that combines [`AssetLoadBlockAppReady`], [`AssetPlaceholder`], and [`RunOnAppReady`].
#[derive(Bundle)]
pub struct AssetRunOnReady<A: Asset> {
	pub block_asset_ready: AssetLoadBlockAppReady,
	pub placeholder: AssetPlaceholder<A>,
	pub run_on_ready: RunOnAppReady,
}
impl<A: Asset> AssetRunOnReady<A> {
	pub fn new(path: impl Into<String>) -> Self {
		Self {
			block_asset_ready: AssetLoadBlockAppReady,
			placeholder: AssetPlaceholder::new(path),
			run_on_ready: RunOnAppReady::default(),
		}
	}
}
