//! Syncing one store into another from markup.
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// Make the store `dest` hold what the store `src` holds, copying only what
/// differs by size and digest ([`BlobSync`]). The one sync verb: each end is a
/// [`StoreEnd`], and which end is which is the direction, so a push and a pull
/// of one bucket are the two orders of the same tag:
///
/// ```bsx
/// <S3BucketBlock bx:ref="assets" label="assets" deploy_versioned=false/>
/// <StoreSync src="fs:site/assets" dest={$assets}/>
/// <StoreSync src={$assets} dest="fs:site/assets" anonymous=true/>
/// ```
///
/// A relative `fs:` uri roots at the workspace, so the declaration reads like
/// every other workspace path. A declared end is the real store, never the
/// local form the launch's [`ServiceAccess`] may have attached beside it, since
/// a deploy launched with local access must still reach the bucket: a store
/// block is its remote root, resolved through its own stack, its deploy's
/// folder when `deploy_versioned` (the launch's version, which
/// [`AdoptCurrentDeploy`] points at the live one) and its storage class when
/// written to. Any other declaration is exactly the store on its entity.
///
/// Additive by default. `delete` opts into a *mirror*, where objects absent
/// from the source are pruned so the destination exactly reflects it: a deploy
/// wants that (a renamed or removed source file otherwise lingers across
/// deploys, eg a home route converted `index.bsx` -> `index.md` leaves the
/// stale `index.bsx`, so the served binary sees two routes for `/` and panics
/// on boot), a source of record does not. A mirror of an empty or missing
/// source is refused.
#[action]
#[derive(Debug, Default, Component, Reflect, MapEntities)]
#[reflect(Component, Default, MapEntities)]
pub async fn StoreSync(
	/// The store read from, a uri or a declaration.
	#[field(required)]
	#[entities]
	src: StoreEnd,
	/// The store written to, a uri or a declaration.
	#[field(required)]
	#[entities]
	dest: StoreEnd,
	/// A folder of the source to sync from, its root by default.
	#[field]
	src_dir: Option<FwdPath>,
	/// A folder of the destination to sync into, its root by default.
	#[field]
	dest_dir: Option<FwdPath>,
	/// Prune destination objects absent from the source (a mirror rather than
	/// an additive sync).
	#[field]
	delete: bool,
	/// Compare the two ends by SIZE alone, rather than by size and digest.
	///
	/// Right for a store whose content is immutable per path (a segment log, a
	/// content-addressed blob), where size is already a complete comparison and
	/// spares reading every local file to digest it. Wrong for a mutable
	/// mirror, where an edit that happened to keep the byte count would be
	/// skipped.
	#[field]
	size_only: bool,
	/// Report what would move without moving it.
	#[field]
	dry_run: bool,
	/// Narrow the sync to part of the ends, ie
	/// `filter={{exclude:["*/blobs/*"]}}` to hydrate a store's records without
	/// its media. Empty by default, everything. Matched against each path below
	/// the two ends, so a pattern reads the same in either direction, and a
	/// path it refuses is neither copied nor pruned.
	#[field]
	filter: GlobFilter,
	/// Read through the symbolic links of a filesystem end rather than skipping
	/// them, so a linked directory is materialized as the store's own. Refused
	/// where neither end is a filesystem.
	#[field]
	follow_symlinks: bool,
	/// Read the source unsigned, for a public bucket and a launch holding no
	/// credential. Refused where the source is not S3.
	#[field]
	anonymous: bool,
	cx: ActionContext<Request>,
) -> Result<Outcome<Request, Response>> {
	let src = StoreSync::resolve(&cx.caller, src).await?;
	let dest = StoreSync::resolve(&cx.caller, dest).await?;
	// a class declared on the store lands new objects there directly, sparing
	// each a day at the default class and a transition request
	#[cfg(feature = "aws")]
	let dest_end = dest
		.end
		.with_storage_class(dest.storage_class.map(|class| class.as_str()));
	#[cfg(not(feature = "aws"))]
	let dest_end = dest.end;
	StoreMirror::new(src.end.with_dir(src_dir), dest_end.with_dir(dest_dir))
		.with_delete(delete)
		.with_size_only(size_only)
		.with_dry_run(dry_run)
		.with_filter(filter)
		.with_follow_symlinks(follow_symlinks)
		.with_anonymous(anonymous)
		.run()
		.await?;
	Pass(cx.input).xok()
}

/// A [`StoreEnd`] resolved as the sync runs, rather than as it is declared:
/// which deploy's folder it is belongs to the verb, not the declaration.
struct ResolvedEnd {
	end: SyncEnd,
	/// The class a write into it lands objects in, where its block declares
	/// one.
	#[cfg_attr(not(feature = "aws"), allow(dead_code))]
	storage_class: Option<S3StorageClass>,
}

impl StoreSync {
	/// The store `end` names, resolved for the sync on `caller`.
	async fn resolve(
		caller: &AsyncEntity,
		end: StoreEnd,
	) -> Result<ResolvedEnd> {
		let target = match end {
			StoreEnd::Uri(uri) => {
				return ResolvedEnd {
					end: SyncEnd::from_uri(
						&uri.rooted_at(&AbsPath::workspace_root()),
					)?,
					storage_class: None,
				}
				.xok();
			}
			StoreEnd::Store(target) => target,
		};
		let block = caller
			.world()
			.entity(target)
			.with_state::<(StackQuery, Query<(&ErasedBlock, &ErasedStoreBlock)>), _>(
				|target, (stacks, blocks)| -> Result<_> {
					let Ok((erased, block)) = blocks.get(target) else {
						return None.xok();
					};
					if *block.kind() == StoreKind::Table {
						bevybail!(
							"`{}` is declared a table store: a sync is of blobs",
							erased.label
						);
					}
					// the stack as this query sees it rather than as the
					// declaration landed, so a render at another stage names that
					// stage's store
					let block = block.at(&stacks.resolve(target))?;
					Some((
						block.store_uri(Some(&stacks.deploy_id())),
						*block.storage_class(),
					))
					.xok()
				},
			)
			.await??;
		match block {
			Some((uri, storage_class)) => ResolvedEnd {
				end: SyncEnd::from_uri(&uri)?,
				storage_class,
			},
			None => ResolvedEnd {
				end: SyncEnd::declared(caller.world(), target).await?,
				storage_class: None,
			},
		}
		.xok()
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_action::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;
	use beet_scene::prelude::*;

	fn world() -> World {
		let mut world = (
			AsyncPlugin,
			BootstrapPlugin,
			ScenePlugin,
			DocumentPlugin,
			InfraPlugin,
		)
			.into_world();
		world.init_resource::<PackageConfig>();
		world
	}

	/// Run one sync as the sequence it would be declared under calls it.
	async fn run(sync: StoreSync) -> Result {
		world()
			.spawn(sync)
			.call::<Request, Outcome<Request, Response>>(Request::get(""))
			.await?;
		().xok()
	}

	fn uri(uri: &str) -> Option<StoreEnd> {
		Some(StoreUri::parse(uri).unwrap().into())
	}

	async fn listed(store: &BlobStore) -> Vec<FwdPath> {
		let mut paths = store.list().await.unwrap();
		paths.sort();
		paths
	}

	/// The `prod` stack of `app` around `markup`, the sync in it its last
	/// element.
	fn declared(markup: &str) -> (World, Entity) {
		let mut world = world();
		BsxParser::spawn(
			&mut world,
			&format!(
				r#"<Stack app_name="app" stage="prod" {{AwsRegion("eu-west-1")}}>{markup}</Stack>"#
			),
		)
		.unwrap();
		world.flush();
		let sync = world.query_filtered_once::<Entity, With<StoreSync>>()[0];
		(world, sync)
	}

	/// Each end of the declared sync as it resolves.
	async fn resolved(markup: &str) -> Result<(String, String)> {
		let (mut world, sync) = declared(markup);
		let declared = world.get::<StoreSync>(sync).unwrap();
		let (src, dest) = (
			declared.src.clone().unwrap(),
			declared.dest.clone().unwrap(),
		);
		world
			.run_async_then(move |world| async move {
				let sync = world.entity(sync);
				(
					StoreSync::resolve(&sync, src).await?.end.to_string(),
					StoreSync::resolve(&sync, dest).await?.end.to_string(),
				)
					.xok()
			})
			.await
	}

	/// An additive sync leaves what the source lacks in place, and `delete`
	/// prunes it, so only a mirror can destroy destination state. A relative
	/// `fs:` uri is workspace relative.
	#[beet_core::test]
	async fn mirrors_additively_then_prunes_on_delete() {
		let dir = TempDir::new_ws().unwrap();
		let (src, dest) = (dir.path().join("src"), dir.path().join("dest"));
		fs_ext::write(src.join("a.txt"), "a").unwrap();
		fs_ext::write(src.join("nested/b.txt"), "b").unwrap();
		fs_ext::write(dest.join("stale.txt"), "s").unwrap();
		let target = BlobStore::new(FsStore::new(dest.clone()));
		let sync = |delete: bool| StoreSync {
			src: uri(&format!("fs:{}", src.clone().into_ws_path().unwrap())),
			dest: uri(&format!("fs:{}", dest.clone().into_ws_path().unwrap())),
			delete,
			..default()
		};

		run(sync(false)).await.unwrap();
		listed(&target).await.xpect_eq(vec![
			fwd_path!("a.txt"),
			fwd_path!("nested/b.txt"),
			fwd_path!("stale.txt"),
		]);

		run(sync(true)).await.unwrap();
		listed(&target).await.xpect_eq(vec![
			fwd_path!("a.txt"),
			fwd_path!("nested/b.txt"),
		]);
	}

	/// Every option reaches the sync: folders, the filter, size alone and the
	/// dry run.
	#[beet_core::test]
	async fn options_reach_the_sync() {
		let name = |part: &str| format!("memory://store-sync-options-{part}");
		let store = |part: &str| {
			BlobStore::from_uri(&StoreUri::parse(&name(part)).unwrap()).unwrap()
		};
		let (src, dest) = (store("src"), store("dest"));
		for (path, body) in [
			("in/a.txt", "new"),
			("in/blobs/b.png", "b"),
			("out.txt", "o"),
		] {
			src.insert(&FwdPath::new(path).unwrap(), body).await.unwrap();
		}
		dest.insert(&fwd_path!("to/a.txt"), "old").await.unwrap();
		let sync = || StoreSync {
			src: uri(&name("src")),
			dest: uri(&name("dest")),
			src_dir: Some(fwd_path!("in")),
			dest_dir: Some(fwd_path!("to")),
			filter: GlobFilter::default().with_exclude("blobs/*"),
			..default()
		};
		let read = async || dest.get(&fwd_path!("to/a.txt")).await.unwrap();
		for skipped in [
			StoreSync {
				dry_run: true,
				..sync()
			},
			StoreSync {
				size_only: true,
				..sync()
			},
		] {
			run(skipped).await.unwrap();
			read().await.xpect_eq(bytes::Bytes::from("old"));
		}
		run(sync()).await.unwrap();
		read().await.xpect_eq(bytes::Bytes::from("new"));
		listed(&dest).await.xpect_eq(vec![fwd_path!("to/a.txt")]);
	}

	/// An option for another kind of end, and a mirror of nothing, are refused.
	#[beet_core::test]
	async fn refuses_what_means_nothing() {
		let sync = || StoreSync {
			src: uri("memory://store-sync-refuses-src"),
			dest: uri("memory://store-sync-refuses-dest"),
			..default()
		};
		for (sync, reason) in [
			(
				StoreSync {
					anonymous: true,
					..sync()
				},
				"unsigned read of an S3 source",
			),
			(
				StoreSync {
					follow_symlinks: true,
					..sync()
				},
				"for a filesystem end",
			),
			(
				StoreSync {
					delete: true,
					..sync()
				},
				"memory:store-sync-refuses-src is missing or empty",
			),
			(
				StoreSync {
					src: None,
					..sync()
				},
				"src",
			),
		] {
			run(sync)
				.await
				.unwrap_err()
				.to_string()
				.xpect_contains(reason);
		}
	}

	/// A declared bucket is the bucket its own stack names, whatever the
	/// launch's service access, which locally attaches a directory beside it:
	/// a sync cannot address one the deploy did not create.
	#[beet_core::test]
	async fn a_declared_bucket_is_the_real_one() {
		resolved(
			r#"<S3BucketBlock bx:ref="assets" label="assets" deploy_versioned=false/>
			<StoreSync src="fs:site/assets" dest={$assets}/>"#,
		)
		.await
		.unwrap()
		.xpect_eq((
			format!("fs:{}", WsPath::from(fwd_path!("site/assets")).into_abs()),
			"s3://app--prod--assets?region=eu-west-1".into(),
		));
		// a pull is the other order, and a forward reference resolves
		resolved(
			r#"<StoreSync src={$assets} dest="fs:site/assets" anonymous=true/>
			<S3BucketBlock bx:ref="assets" label="assets" deploy_versioned=false/>"#,
		)
		.await
		.unwrap()
		.0
		.xpect_eq("s3://app--prod--assets?region=eu-west-1");
	}

	/// The bucket's OWN stack names it, not the sync's: one declared under
	/// another stack and region is addressed there.
	#[beet_core::test]
	async fn a_bucket_of_another_stack_is_addressed_there() {
		resolved(
			r#"<Stack app_name="other" stage="shared" {AwsRegion("ap-southeast-2")}>
				<S3BucketBlock bx:ref="assets" label="assets" deploy_versioned=false/>
			</Stack>
			<StoreSync src="fs:site/assets" dest={$assets}/>"#,
		)
		.await
		.unwrap()
		.1
		.xpect_eq("s3://other--shared--assets?region=ap-southeast-2");
	}

	/// A deploy-versioned store is synced below the version the launch
	/// addresses, its document folder.
	#[beet_core::test]
	async fn a_versioned_store_nests_the_deploy() {
		let (mut world, sync) = declared(
			r#"<S3BucketBlock bx:ref="package" label="package"/>
			<StoreSync src="fs:site" dest={$package}/>"#,
		);
		let dest = world.get::<StoreSync>(sync).unwrap().dest.clone().unwrap();
		let deploy_id = *world.resource::<Deployment>().deploy_id();
		world
			.run_async_then(move |world| async move {
				StoreSync::resolve(&world.entity(sync), dest)
					.await
					.map(|end| end.end.to_string())
			})
			.await
			.unwrap()
			.xpect_eq(format!(
				"s3://app--prod--package/{deploy_id}/package?region=eu-west-1"
			));
	}

	/// The class a write lands objects in is the bucket's own declaration, and
	/// absent for a bucket declaring none.
	#[beet_core::test]
	async fn the_declared_class_reaches_the_sync() {
		for (attr, expected) in [
			("", None),
			(
				r#"storage_class="GlacierIr""#,
				Some(S3StorageClass::GlacierIr),
			),
		] {
			let (mut world, sync) = declared(&format!(
				r#"<S3BucketBlock bx:ref="cold" label="cold" deploy_versioned=false {attr}/>
				<StoreSync src="fs:store" dest={{$cold}}/>"#
			));
			let dest =
				world.get::<StoreSync>(sync).unwrap().dest.clone().unwrap();
			world
				.run_async_then(move |world| async move {
					StoreSync::resolve(&world.entity(sync), dest)
						.await
						.map(|end| end.storage_class)
				})
				.await
				.unwrap()
				.xpect_eq(expected);
		}
	}

	/// Any other declaration is exactly the store on its entity, its path
	/// spelled once, and one naming no blob store is an error, never a sync of
	/// something else.
	#[beet_core::test]
	async fn any_other_declaration_is_its_own_store() {
		resolved(
			r#"<FsStore bx:ref="store" path="store"/>
			<StoreSync src={$store} dest="memory://d"/>"#,
		)
		.await
		.unwrap()
		.0
		.xpect_eq(format!("fs:{}", WsPath::from(fwd_path!("store")).into_abs()));
		resolved(
			r#"<DynamoTableBlock bx:ref="table" label="table"/>
			<StoreSync src={$table} dest="memory://d"/>"#,
		)
		.await
		.unwrap_err()
		.to_string()
		.xpect_contains("`table` is declared a table store");
		resolved(
			r#"<div bx:ref="nothing"/>
			<StoreSync src={$nothing} dest="memory://d"/>"#,
		)
		.await
		.unwrap_err()
		.to_string()
		.xpect_contains("BlobStore");
	}
}
