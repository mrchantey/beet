//! Bind a local directory to a declared bucket, for [`SyncS3Bucket`] to sync.

use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// `<DirSync bucket="app" local_dir="site"/>` — the local end and the bucket
/// end of one sync, addressed by bucket *label* so the
/// `<app>--<stage>--<label>` name composes through the same [`Stack`] the
/// deploy created the bucket with.
///
/// The sync itself is [`SyncS3Bucket`], required here and overridable as a
/// colocated spread, ie `<DirSync bucket="app" local_dir="site"
/// {SyncS3Bucket{delete:true}}/>`. The ends are named by where they are, not by
/// their role, since the direction flips which one is the source: `local_dir` is
/// workspace-relative.
#[derive(Debug, Clone, Get, SetWith, Component, Reflect)]
#[reflect(Component, Default)]
#[require(SyncS3Bucket)]
pub struct DirSync {
	/// The bucket's declared label, ie `app` or `assets`.
	bucket: SmolStr,
	/// The workspace-relative local directory.
	local_dir: SmolPath,
	/// Override the resolved stage, for a bucket outside the deploy's own stage.
	#[set_with(unwrap_option, into)]
	stage: Option<SmolStr>,
	/// Override the region the bucket lives in, which otherwise resolves from
	/// the ancestor [`Stack`].
	#[set_with(unwrap_option, into)]
	region: Option<SmolStr>,
}

impl Default for DirSync {
	fn default() -> Self { Self::new("", "") }
}

impl DirSync {
	pub fn new(
		bucket: impl Into<SmolStr>,
		local_dir: impl Into<SmolPath>,
	) -> Self {
		Self {
			bucket: bucket.into(),
			local_dir: local_dir.into(),
			stage: None,
			region: None,
		}
	}

	/// The stack this sync addresses its bucket in: the entity's resolved stack,
	/// with the [`stage`](Self::stage) and [`region`](Self::region) overrides
	/// applied.
	fn stack(&self, resolved: Stack) -> Stack {
		let resolved = match &self.stage {
			Some(stage) => resolved.with_stage(stage.clone()),
			None => resolved,
		};
		match &self.region {
			Some(region) => resolved.with_region(region.clone()),
			None => resolved,
		}
	}
}

/// Observer: resolve the declared bucket into the [`S3FsStore`]
/// [`SyncS3BucketAction`] reads. Deferred through the command queue because the
/// ancestry a scope resolves against lands with the rest of the scene.
#[cfg(all(feature = "aws_sdk", not(target_arch = "wasm32")))]
pub(crate) fn attach_dir_sync_store(
	ev: On<Add, DirSync>,
	mut commands: Commands,
) {
	commands
		.entity(ev.entity)
		.queue(|mut entity: EntityWorldMut| -> Result {
			let sync = entity.get_or_else::<DirSync>()?.clone();
			let stack = entity
				.with_state::<StackQuery, _>(|entity, stacks| {
					stacks.resolve(entity)
				})
				.xmap(|stack| sync.stack(stack));
			entity.insert(S3FsStore::new(
				FsStore::new(WsPathBuf::new(sync.local_dir().to_string())),
				S3Store::new(
					stack.resource_name(sync.bucket().clone()),
					stack.region(),
				),
			));
			Ok(())
		});
}
