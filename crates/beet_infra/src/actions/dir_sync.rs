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
	/// Override the resolved stage, for a bucket outside the deploy's own
	/// stage. A bucket in another region carries an `{AwsRegion(..)}` spread
	/// on this entity, resolved by ancestry like every address.
	#[set_with(unwrap_option, into)]
	stage: Option<SmolStr>,
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
		}
	}

	/// The stack this sync addresses its bucket in: the entity's resolved stack,
	/// with the [`stage`](Self::stage) override applied. Still resolved: an
	/// override replaces an answer, never unsets it.
	pub fn stack(&self, resolved: ResolvedStack) -> ResolvedStack {
		match &self.stage {
			Some(stage) => resolved.with_stage(stage.clone()),
			None => resolved,
		}
	}
}

/// The declaration of the bucket this sync addresses: the [`ErasedStoreBlock`]
/// under the stack carrying the sync's label. What the sync reads off it (the
/// per-deploy prefix, the storage class) comes from the bucket's own
/// declaration rather than from a field on the sync, so a sync cannot
/// disagree with the bucket it addresses. A label nothing under the stack
/// declares is an error: the bucket name would compose fine and the sync would
/// publish into thin air.
///
/// Looked up when the sync RUNS rather than when it is declared, because which
/// deploy's prefix a sync belongs to is a property of the VERB and not of the
/// declaration: `deploy` mints a version and fills it, while `sync` republishes
/// into the version already being served (`<AdoptCurrentDeploy/>` is what points
/// the launch at it). One declaration, read by both.
#[cfg(all(feature = "aws_sdk", not(target_arch = "wasm32")))]
pub(crate) fn declared_store<'a>(
	entity: Entity,
	sync: &DirSync,
	stacks: &StackQuery,
	stores: &'a Query<(&ErasedBlock, &ErasedStoreBlock)>,
) -> Result<&'a ErasedStoreBlock> {
	stacks
		.declared(entity)?
		.into_iter()
		.filter_map(|entity| stores.get(entity).ok())
		.find(|(erased, _)| erased.label == *sync.bucket())
		.map(|(_, store)| store)
		.ok_or_else(|| {
			bevyhow!(
				"the sync of '{}' addresses a bucket labelled '{}', which \
				 nothing under this stack declares",
				sync.local_dir(),
				sync.bucket()
			)
		})
}

/// Observer: resolve the declared bucket into the [`S3FsStore`]
/// [`SyncS3Bucket`] reads. Deferred through the command queue because the
/// ancestry a scope resolves against lands with the rest of the scene.
///
/// The bucket IDENTITY only (its name and region), spelled by the declaration
/// ([`S3BucketBlock::store_uri`]) rather than recomposed here: a throwaway
/// block under the sync's own stack, since an overridden `stage` or a region
/// spread on the sync addresses a bucket under another stack, which no label
/// lookup here can reach. The per-deploy prefix a versioned bucket nests
/// under is resolved from [`declared_store`] when the sync runs, since it is
/// not yet known here.
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
				S3BucketBlock::new(sync.bucket().clone()).store_uri(&stack)?;
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

	/// The `prod` stage of `app`, addressed in `eu-west-1`.
	fn prod_stack() -> impl Bundle {
		(
			Stack::new("app").with_stage("prod"),
			AwsRegion::new("eu-west-1"),
		)
	}

	/// A stack declaring `bucket` beside a sync of it, returning the sync's
	/// entity and the declaration's erased half.
	fn declared_sync(
		stack: impl Bundle,
		bucket: S3BucketBlock,
		sync: impl Bundle,
	) -> (World, Entity, ErasedStoreBlock) {
		let mut world = InfraPlugin.into_world();
		world.init_resource::<PackageConfig>();
		let stack = world.spawn((stack, children![bucket, sync])).id();
		world.flush();
		let children = world.entity(stack).get::<Children>().unwrap();
		let (bucket, sync) = (children[0], children[1]);
		let declared = world.get::<ErasedStoreBlock>(bucket).unwrap().clone();
		(world, sync, declared)
	}

	/// The attached S3 end is the bucket the declaration names, spelled by the
	/// same [`StoreBlock::store_uri`], so the sync cannot address a bucket the
	/// deploy did not create.
	#[beet_core::test]
	fn attaches_the_declared_bucket() {
		let (world, sync, declared) = declared_sync(
			prod_stack(),
			S3BucketBlock::new("assets"),
			DirSync::new("assets", "site"),
		);
		let attached = world.get::<S3FsStore>(sync).unwrap().s3_store();
		declared.root().name().xpect_eq(Some("app--prod--assets"));
		attached
			.bucket_name()
			.as_str()
			.xpect_eq(declared.root().name().unwrap());
		attached
			.region()
			.clone()
			.xpect_eq(Some(SmolStr::new("eu-west-1")));
	}

	/// A `stage` override and a region spread on the sync address the same
	/// label under another stack, composed through the same declaration.
	#[beet_core::test]
	fn overrides_address_another_stack() {
		let (world, sync, _) = declared_sync(
			prod_stack(),
			S3BucketBlock::new("assets"),
			(
				DirSync::new("assets", "site").with_stage("shared"),
				AwsRegion::new("ap-southeast-2"),
			),
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

	/// The class a push lands objects in is the bucket's own declaration,
	/// projected onto the erased half the sync reads, and absent for a bucket
	/// declaring none.
	#[beet_core::test]
	fn the_declared_class_reaches_the_sync() {
		let (_, _, declared) = declared_sync(
			prod_stack(),
			S3BucketBlock::new("archive")
				.with_storage_class(S3StorageClass::GlacierIr),
			DirSync::new("archive", "store"),
		);
		declared
			.storage_class()
			.xpect_eq(Some(S3StorageClass::GlacierIr));
		let (_, _, declared) = declared_sync(
			prod_stack(),
			S3BucketBlock::new("assets"),
			DirSync::new("assets", "site"),
		);
		declared.storage_class().xpect_eq(None);
	}
}
