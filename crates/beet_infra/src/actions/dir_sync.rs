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
	local_dir: WsPath,
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
		local_dir: impl Into<WsPath>,
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
	/// applied. Still resolved: an override replaces an answer, never unsets it.
	pub fn stack(&self, resolved: ResolvedStack) -> ResolvedStack {
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

/// The per-deploy prefix this sync publishes under, or `None` for a bucket that
/// publishes at its root.
///
/// Resolved when the sync RUNS rather than when it is declared, because which
/// deploy's prefix a sync belongs to is a property of the VERB and not of the
/// declaration: `deploy` mints a version and fills it, while `sync` republishes
/// into the version already being served (`<AdoptCurrentDeploy/>` is what points
/// the launch at it). One declaration, read by both.
///
/// The flag comes from the bucket's own declaration (its
/// [`ErasedStoreBlock`]) rather than from a field here, so a sync cannot
/// disagree with the bucket it addresses. A label nothing under the stack
/// declares is an error: the bucket name would compose fine and the sync would
/// publish into thin air.
#[cfg(all(feature = "aws_sdk", not(target_arch = "wasm32")))]
pub(crate) fn deploy_subdir(
	entity: Entity,
	sync: &DirSync,
	stacks: &StackQuery,
	stores: &Query<(&ErasedBlock, &ErasedStoreBlock)>,
) -> Result<Option<RelPath>> {
	let (_, store) = stacks
		.declared(entity)?
		.into_iter()
		.filter_map(|entity| stores.get(entity).ok())
		.find(|(erased, _)| erased.label == *sync.bucket())
		.ok_or_else(|| {
			bevyhow!(
				"the sync of '{}' addresses a bucket labelled '{}', which \
				 nothing under this stack declares",
				sync.local_dir(),
				sync.bucket()
			)
		})?;
	match store.deploy_versioned() {
		true => Some(RelPath::new(stacks.deploy_id().to_string())),
		false => None,
	}
	.xok()
}

/// Observer: resolve the declared bucket into the [`S3FsStore`]
/// [`SyncS3Bucket`] reads. Deferred through the command queue because the
/// ancestry a scope resolves against lands with the rest of the scene.
///
/// The bucket IDENTITY only (its name and region), spelled by the declaration
/// ([`S3BucketBlock::store_uri`]) rather than recomposed here: a throwaway
/// block under the sync's own stack, since an overridden `stage`/`region`
/// addresses a bucket under another stack, which no label lookup here can
/// reach. The per-deploy prefix a versioned bucket nests under is resolved by
/// [`deploy_subdir`] when the sync runs, since it is not yet known here.
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
			let uri =
				S3BucketBlock::new(sync.bucket().clone()).store_uri(&stack);
			entity.insert(S3FsStore::new(
				FsStore::new(sync.local_dir()),
				S3Store::from_uri(&uri)?,
			));
			Ok(())
		});
}

#[cfg(all(test, feature = "aws_sdk", not(target_arch = "wasm32")))]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;

	/// A stack declaring `bucket` beside a sync of it, returning the sync's
	/// entity and the declaration's root uri.
	fn declared_sync(
		stack: Stack,
		bucket: S3BucketBlock,
		sync: DirSync,
	) -> (World, Entity, StoreUri) {
		let mut world = InfraPlugin.into_world();
		world.init_resource::<PackageConfig>();
		let stack = world.spawn((stack, children![bucket, sync])).id();
		world.flush();
		let children = world.entity(stack).get::<Children>().unwrap();
		let (bucket, sync) = (children[0], children[1]);
		let root = world
			.get::<ErasedStoreBlock>(bucket)
			.unwrap()
			.root()
			.clone();
		(world, sync, root)
	}

	/// The attached S3 end is the bucket the declaration names, spelled by the
	/// same [`StoreBlock::store_uri`], so the sync cannot address a bucket the
	/// deploy did not create.
	#[beet_core::test]
	fn attaches_the_declared_bucket() {
		let (world, sync, root) = declared_sync(
			Stack::new("app")
				.with_stage("prod")
				.with_region("eu-west-1"),
			S3BucketBlock::new("assets"),
			DirSync::new("assets", "site"),
		);
		let attached = world.get::<S3FsStore>(sync).unwrap().s3_store();
		root.name().xpect_eq(Some("app--prod--assets"));
		attached
			.bucket_name()
			.as_str()
			.xpect_eq(root.name().unwrap());
		attached
			.region()
			.clone()
			.xpect_eq(Some(SmolStr::new("eu-west-1")));
	}

	/// A `stage`/`region` override addresses the same label under another
	/// stack, composed through the same declaration.
	#[beet_core::test]
	fn overrides_address_another_stack() {
		let (world, sync, _) = declared_sync(
			Stack::new("app").with_stage("prod"),
			S3BucketBlock::new("assets"),
			DirSync::new("assets", "site")
				.with_stage("shared")
				.with_region("ap-southeast-2"),
		);
		let attached = world.get::<S3FsStore>(sync).unwrap().s3_store();
		attached
			.bucket_name()
			.as_str()
			.xpect_eq("app--shared--assets");
		attached
			.region()
			.clone()
			.xpect_eq(Some(SmolStr::new("ap-southeast-2")));
	}
}
