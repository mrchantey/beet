use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// Syncs a local directory and a declared bucket, in either direction.
///
/// Both ends come from the nearest ancestor [`DirSync`] (or, where a store was
/// spawned directly, from its [`S3FsStore`]): the bucket end is the identity
/// the declaration composed, the local end is the sync's `local_dir` or the
/// store a colocated `StoreRef` names.
///
/// The defaults are the conservative ones: push, additive. `delete` opts into a
/// *mirror*, where objects absent from the source are pruned so the destination
/// exactly reflects it. A deploy wants that (a renamed or removed source file
/// otherwise lingers across deploys, eg a home route converted `index.bsx` ->
/// `index.md` leaves the stale `index.bsx`, so the served binary sees two routes
/// for `/` and panics on boot), a source of record does not.
#[action]
#[derive(Debug, Default, Component, Reflect)]
#[reflect(Component, Default)]
pub async fn SyncS3Bucket(
	/// Which end is the source.
	#[field]
	direction: SyncDirection,
	/// Prune destination entries absent from the source (a mirror rather than an
	/// additive sync). Guarded on push by [`SyncS3Bucket::assert_mirrorable`].
	#[field]
	delete: bool,
	/// Compare the two ends by SIZE alone, rather than by size and modified
	/// time.
	///
	/// Right for a store whose content is immutable per path (a segment log, a
	/// content-addressed blob), where size is already a complete comparison and
	/// the default makes a hydrate permanent: a downloaded file is always newer
	/// than the object it came from, so the next push re-uploads the whole
	/// store. Wrong for a mutable mirror, where an edit that happened to keep
	/// the byte count would be skipped.
	#[field]
	size_only: bool,
	/// Upload the targets of symbolic links rather than skipping them, so a
	/// symlinked subdir is materialized into the bucket.
	#[field]
	follow_symlinks: bool,
	/// Sync without credentials, for a public-read bucket.
	#[field]
	no_sign_request: bool,
	/// Optional subdir of the bucket to sync against; the bucket root by default.
	#[field]
	bucket_dir: Option<RelPath>,
	/// Narrow the sync to part of the directory, ie
	/// `{filter:{exclude:["*/blobs/*"]}}` to hydrate a store's records without
	/// its media. Empty by default, the whole directory. Translated into the
	/// cli's own rules by [`S3Sync::filter_args`], and matched against each
	/// entry's path below the two ends, so a pattern reads the same in either
	/// direction. A filtered PUSH leaves what it skipped out of the bucket,
	/// which only a store that is not the source of record should want.
	#[field]
	filter: GlobFilter,
	cx: ActionContext<Request>,
) -> Result<Outcome<Request, Response>> {
	trace!("SyncS3Bucket: starting");
	let ends = SyncEnds::declared(&cx.caller).await?;
	let local_dir = ends.local_dir(cx.caller.world()).await?;
	let storage_class = ends.storage_class;
	let s3_store = match ends.deploy_subdir {
		Some(subdir) => ends.s3_store.with_subdir(subdir),
		None => ends.s3_store,
	};
	// `s3_uri` already ends in a separator
	let s3_uri = match &bucket_dir {
		Some(dir) => format!("{}{dir}", s3_store.s3_uri()),
		None => s3_store.s3_uri(),
	};
	// only a mirroring push can destroy remote state, so only it is guarded
	if delete && direction == SyncDirection::Push {
		SyncS3Bucket::assert_mirrorable(&local_dir, follow_symlinks)?;
	}
	if !filter.is_empty() {
		// a pull that silently skipped most of the bucket is the kind of
		// surprise worth a line of output
		info!("SyncS3Bucket: filtered sync: {filter}");
	}
	trace!(
		"SyncS3Bucket: syncing {} {} {s3_uri}",
		local_dir,
		match direction {
			SyncDirection::Push => "->",
			SyncDirection::Pull => "<-",
		}
	);
	match direction {
		SyncDirection::Push => {
			let sync = S3Sync::push(local_dir.clone(), &s3_uri);
			// a class declared on the bucket lands new objects there directly,
			// sparing each a day at Standard and a transition request
			match storage_class {
				Some(class) => sync.storage_class(class.as_str()),
				None => sync,
			}
		}
		SyncDirection::Pull => S3Sync::pull(&s3_uri, local_dir.clone()),
	}
	.filter(filter)
	.delete(delete)
	.size_only(size_only)
	.follow_symlinks(follow_symlinks)
	.no_sign_request(no_sign_request)
	.send()
	.await?;
	trace!("synced {s3_uri} (region: {:?})", s3_store.region());
	trace!("SyncS3Bucket: complete");
	Pass(cx.input).xok()
}

/// What one sync resolves when it RUNS rather than when it is declared,
/// because each answer belongs to the verb and not to the declaration: which
/// deploy's prefix it publishes into (see `dir_sync::declared_store`), what
/// class the bucket lands its objects in, and which directory is its local
/// end.
struct SyncEnds {
	/// The bucket end, composed by the declaration this sync addresses.
	s3_store: S3Store,
	/// The local end where the sync spells a path (`local_dir="store"`), or a
	/// store was spawned directly with the root it means to publish into.
	local_root: Option<AbsPath>,
	/// The declaration naming the local end, where a `StoreRef` names one
	/// instead of a path.
	local_ref: Option<Entity>,
	/// The per-deploy prefix a deploy-versioned bucket publishes under.
	deploy_subdir: Option<RelPath>,
	/// The class the bucket's own declaration lands new objects in.
	storage_class: Option<S3StorageClass>,
}

impl SyncEnds {
	/// Everything the caller's ancestry says about this sync, under one
	/// exclusive read.
	async fn declared(caller: &AsyncEntity) -> Result<Self> {
		caller
			.with_state::<(
				AncestorQuery<AnyOf<(&S3FsStore, &S3Store)>>,
				AncestorQuery<(&DirSync, Option<&StoreRef>)>,
				StackQuery,
				Query<(&ErasedBlock, &ErasedStoreBlock)>,
			), _>(|entity, (stores, syncs, stacks, declared)| -> Result<_> {
				// both ends on one entity is a `<DirSync local_dir=..>` or a
				// store spawned directly; the bucket end alone is a
				// `<DirSync>` whose local end is a declaration
				let (s3_store, local_root) = match stores.get(entity)? {
					(Some(store), _) => (
						store.s3_store().clone(),
						Some(store.fs_store().effective_root()),
					),
					(None, Some(s3_store)) => (s3_store.clone(), None),
					(None, None) => {
						bevybail!("entity {entity} has neither store")
					}
				};
				// a store spawned directly (rather than through `<DirSync>`)
				// already carries whatever root it means to publish into, and
				// lands objects in the bucket default
				let Ok((sync, store_ref)) = syncs.get(entity) else {
					return Self {
						s3_store,
						local_root,
						local_ref: None,
						deploy_subdir: None,
						storage_class: None,
					}
					.xok();
				};
				let block = crate::actions::declared_store(
					entity, sync, &stacks, &declared,
				)?;
				Self {
					s3_store,
					local_root,
					local_ref: store_ref.map(StoreRef::store),
					deploy_subdir: block.deploy_versioned().then(|| {
						ArtifactLedger::version_repo_dir(&stacks.deploy_id())
					}),
					storage_class: *block.storage_class(),
				}
				.xok()
			})
			.await?
	}

	/// The local end, resolved AFTER the exclusive read rather than inside it:
	/// a declaration's runtime half lands through the command queue, so only
	/// an async caller can wait for it.
	async fn local_dir(&self, world: &AsyncWorld) -> Result<AbsPath> {
		match self.local_ref {
			Some(target) => StoreRef::resolve::<FsStore>(world, target)
				.await?
				.effective_root()
				.xok(),
			None => self.local_root.clone().ok_or_else(|| {
				bevyhow!(
					"this sync names no local directory: give the `<DirSync>` \
					 a `local_dir`, or a `{{StoreRef($declaration)}}` naming \
					 the store it syncs"
				)
			}),
		}
	}
}

impl SyncS3Bucket {
	/// Refuse to mirror a local dir that would empty the bucket: a missing or
	/// empty source, or (when following symlinks) a symlinked child dir whose
	/// target is missing or empty — the signature of an unhydrated checkout.
	///
	/// Only the push direction destroys remote state, so only push calls this; a
	/// pull with `delete` overwrites a local dir the caller asked for.
	pub fn assert_mirrorable(
		local_dir: &AbsPath,
		follow_symlinks: bool,
	) -> Result {
		if fs_ext::is_dir_empty(local_dir)? {
			bevybail!(
				"refusing to mirror an empty local dir into a bucket: {}\nhydrate it first, eg `just beet-shared pull`",
				local_dir
			);
		}
		if !follow_symlinks {
			return Ok(());
		}
		// a linked child dir is materialized into the bucket by this push, so an
		// unhydrated link is the same emptying hazard one level down.
		for child in ReadDir::all(local_dir)?
			.into_iter()
			.filter(|child| fs_ext::is_symlink(child))
		{
			if fs_ext::is_dir_empty(&child)? {
				bevybail!(
					"refusing to mirror an empty symlinked dir into a bucket: {}\nhydrate it first, eg `just beet-shared pull`",
					child.display()
				);
			}
		}
		Ok(())
	}
}

#[cfg(test)]
mod test {
	use super::*;

	/// A stack declaring `assets` beside `markup`, returning the world and the
	/// sync entity the markup's last child is.
	fn declared(markup: &str) -> (World, Entity) {
		let mut world =
			(AsyncPlugin, TemplatePlugin, DocumentPlugin, InfraPlugin)
				.into_world();
		world.init_resource::<PackageConfig>();
		let root = world
			.spawn(())
			.insert_template(
				BsxTemplate::parse_document(&format!(
					r#"<Stack stage="prod" {{AwsRegion("eu-west-1")}}>
					<S3BucketBlock label="assets" deploy_versioned=false/>
					{markup}
				</Stack>"#
				))
				.unwrap(),
			)
			.unwrap()
			.id();
		world.flush();
		let stack = world.entity(root).get::<Children>().unwrap()[0];
		let sync = *world
			.entity(stack)
			.get::<Children>()
			.unwrap()
			.last()
			.unwrap();
		(world, sync)
	}

	/// The local end resolved from the sync entity, whichever way it is named.
	async fn local_dir(world: &mut World, sync: Entity) -> Result<AbsPath> {
		world
			.run_async_then(move |world| async move {
				let ends = SyncEnds::declared(&world.entity(sync)).await?;
				ends.local_dir(&world).await
			})
			.await
	}

	/// A path names the local end, the form for a directory no declaration
	/// owns.
	#[beet_core::test]
	async fn a_local_dir_names_the_local_end() {
		let (mut world, sync) =
			declared(r#"<DirSync bucket="assets" local_dir="site/assets"/>"#);
		local_dir(&mut world, sync)
			.await
			.unwrap()
			.xpect_eq(WsPath::new("site/assets").into_abs());
	}

	/// A `StoreRef` names it instead, so a declared store's path is spelled
	/// once. Resolved when the sync RUNS: the declaration's `FsStore` lands
	/// through the same command queue the sync's own observer runs on.
	#[beet_core::test]
	async fn a_store_ref_names_the_local_end() {
		let (mut world, sync) = declared(
			r#"<Fragment>
				<FsStore bx:ref="store" path="store"/>
				<DirSync bucket="assets" {StoreRef($store)}/>
			</Fragment>"#,
		);
		// the sync is the fragment's second child, not the stack's last
		let sync = world.entity(sync).get::<Children>().unwrap()[1];
		local_dir(&mut world, sync)
			.await
			.unwrap()
			.xpect_eq(WsPath::new("store").into_abs());
	}

	/// Naming neither is a loud error, not a sync of the workspace root.
	#[beet_core::test]
	async fn naming_neither_end_fails() {
		let (mut world, sync) = declared(r#"<DirSync bucket="assets"/>"#);
		local_dir(&mut world, sync)
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("names no local directory");
	}

	/// A mirroring push refuses an empty source dir, the shape that would empty
	/// the bucket.
	#[beet_core::test]
	fn mirror_guard_rejects_an_empty_dir() {
		let dir = TempDir::new().unwrap();
		let root = (*dir).clone();
		SyncS3Bucket::assert_mirrorable(&root, false).unwrap_err();
		fs_ext::write(root.join("index.html"), "<div/>").unwrap();
		SyncS3Bucket::assert_mirrorable(&root, false).unwrap();
	}

	/// A symlinked child dir is materialized into the bucket by a
	/// `follow_symlinks` mirror, so an unhydrated link is the same hazard one
	/// level down — and only when links are followed.
	#[cfg(unix)]
	#[beet_core::test]
	fn mirror_guard_rejects_an_unhydrated_symlink() {
		let dir = TempDir::new().unwrap();
		let root = (*dir).clone();
		fs_ext::write(root.join("index.html"), "<div/>").unwrap();
		let target = root.join("target");
		fs_ext::create_dir_all(&target).unwrap();
		std::os::unix::fs::symlink(&target, root.join("assets")).unwrap();

		SyncS3Bucket::assert_mirrorable(&root, false).unwrap();
		SyncS3Bucket::assert_mirrorable(&root, true).unwrap_err();
		fs_ext::write(target.join("logo.png"), "png").unwrap();
		SyncS3Bucket::assert_mirrorable(&root, true).unwrap();
	}
}
