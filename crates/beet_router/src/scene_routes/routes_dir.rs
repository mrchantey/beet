//! Runtime route discovery: a directory of content files becomes routes at
//! spawn time, no codegen.
//!
//! Inserting a [`RoutesDir`] (eg from a `main.bsx` entry via
//! `<RoutesDir src="routes"/>`) derives a [`DirPath`] scoping the nearest
//! ancestor [`BlobStore`] (the repo store composed on the loaded root) to `src`
//! onto the dir entity, which also registers the dir's [`WatchDir`] for live
//! reload, and triggers [`RoutesDir::spawn_on_insert`]: the scoped store is
//! listed, and each content file (`.md`/`.mdx`/`.bsx`/`.html`) spawns a
//! [`BlobPage`] route child served through the shared media-parse pipeline,
//! reading its bytes through that store. Each file's ROOT declarations
//! ([`RootDeclarations`]: markdown frontmatter or a BSX root
//! spread) are read at scan time and hoisted onto the route entity, so navigation
//! (eg [`RouteSidebar`](crate::prelude::RouteSidebar)) knows every page's
//! title/order without visiting it. The scan knows no metadata type: it hoists
//! whatever components a document declares, and [`PageMeta`] is one consumer of
//! that set like any other. Discovery is store-backed, so it reads identically
//! from the local filesystem in dev and from S3 in a deployed task.
//!
//! The routes stay true to their files through change detection: each route's
//! [`Blob`] (derived from its [`BlobPage`] path) changing re-reads that file's
//! declarations in place ([`refresh_changed_routes`]), and a file created or
//! removed under the dir marks its scoped store changed, which rescans the dir
//! ([`rescan_changed_dirs`]).

use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_ui::prelude::*;

/// Spawns one [`BlobPage`] route child per content file under `src`,
/// discovered at spawn time (see the module docs).
///
/// Route paths mirror the file tree: `docs/intro.md` serves at `docs/intro`,
/// and an `index.*` file collapses to its directory (`docs/index.md` serves at
/// `docs`). Frontmatter then has the last word on both the url and the ordering:
/// a `slug` renames the final segment and a leading `<number>-` on the filename
/// sets the nav order, so `blog/1-full-stack-bevy.md` declaring
/// `slug = "full-stack-bevy"` reads first and serves at `blog/full-stack-bevy`.
/// Add a [`PathPartial`] alongside to prefix every discovered route.
///
/// A [`filter`](Self::filter) narrows which files are discovered, so one
/// directory can be served by several dirs: the root excludes a subtree another
/// scans under its own [`Route`](crate::prelude::route), giving that subtree its
/// own layout and redirects while the urls stay where they were.
///
/// ```bsx
/// <RoutesDir src="routes" filter={GlobFilter{exclude:["blog/**"]}}/>
/// <Route path="blog" {Layout{template:"ArticleLayout"}}>
///     <RoutesDir src="routes/blog"/>
/// </Route>
/// ```
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component, Default)]
#[component(on_insert = hook_ext::component_hook(|dir: &RoutesDir| DirPath::derive(&dir.src)))]
pub struct RoutesDir {
	/// The content directory, relative to the nearest ancestor [`BlobStore`].
	pub src: RelPath,
	/// Which of `src`'s content files to serve, matched against each file's
	/// path relative to `src` (eg `blog/1-post.md`). Open by default.
	///
	/// A bare string or list authors the include allowlist
	/// (`filter="docs/**"`); the struct literal names either list
	/// (`filter={GlobFilter{exclude:["blog/**"]}}`).
	pub filter: GlobFilter,
}

/// The content file extensions served as [`BlobPage`] routes.
const CONTENT_EXTENSIONS: &[&str] = &["md", "mdx", "markdown", "html", "bsx"];

impl RoutesDir {
	/// Discover routes under `src`, relative to the nearest ancestor [`BlobStore`].
	pub fn new(src: impl Into<RelPath>) -> Self {
		Self {
			src: src.into(),
			..default()
		}
	}

	/// Discover only the files passing `filter`, matched against each path
	/// relative to `src`.
	pub fn with_filter(mut self, filter: GlobFilter) -> Self {
		self.filter = filter;
		self
	}

	/// Observer: scan the [`RoutesDir`] store and spawn its routes (see the module docs).
	///
	/// The scan is store I/O (the filesystem in dev, S3 in a deployed task, R2 in a
	/// Worker), so it runs as an [`AsyncEntity`] task rather than blocking the runtime
	/// (which is single-threaded on wasm). The dir's scoped [`BlobStore`] (its
	/// derived [`DirPath`]'s output, resolved from the nearest ancestor store) is
	/// read *inside* that task, where the whole tree is already built, so it is
	/// reliably present; a store-less app is an error (never an implicit
	/// filesystem store, which has none on wasm).
	///
	/// A rescan (re-inserting the dir, as a live reload does) is a swap, never a
	/// respawn: the new routes spawn hidden beside the old ones and one world
	/// access retires the old set and unhides the new
	/// ([`swap_routes`](Self::swap_routes)), so the tree never holds both, a
	/// request in flight keeps its route until it answers, and no window serves
	/// nothing.
	///
	/// The route children appear a few async ticks after the insert, so the scan
	/// parks a [`PendingGuard`] on the build root (or on this entity outside a
	/// build), deferring [`Ready`] until the routes are spawned. So a load
	/// verb (`CallOnReady`) under the entry root only fans the request out once
	/// every discovered route exists, exactly as the asset / scene deferrals gate
	/// it, and a settle ([`TemplatePending::settle`]) waits on the same set
	/// wherever the dir was inserted.
	pub fn spawn_on_insert(
		ev: On<Insert, RoutesDir>,
		dirs: Query<&RoutesDir>,
		build_root: Option<Res<TemplateBuildRoot>>,
		mut commands: Commands,
	) -> Result {
		let entity = ev.entity;
		let dir = dirs.get(entity)?;
		let src = dir.src.clone();
		let filter = dir.filter.clone();
		let root = build_root.map(|root| **root);
		// one queued command parks the guard (ahead of the build's synchronous
		// drain, like the scene-ready gate) and spawns the scan task holding it,
		// so however the task ends the guard resolves.
		//
		// `run_async_local` (not `run_async`): the scan is bridge-heavy (resolve the
		// ancestor store, then compose it + spawn routes back on the world), and the async
		// bridge only *guarantees* a bridge poll completes when the task runs on the
		// runtime's local executor. A `bevy_multithreaded` build's `spawn` would run it on
		// an `IoTaskPool` worker thread, whose bridge poll can perpetually miss the
		// main-thread world-scope window and stall the scan. Pinning it local keeps
		// discovery deterministic on every target.
		commands.queue(move |world: &mut World| {
			let guard = TemplatePending::park_on(
				world,
				root.unwrap_or(entity),
				PendingKind::Passive,
				format!("<RoutesDir src=\"{src}\"> scan"),
			);
			let Ok(mut entity_mut) = world.get_entity_mut(entity) else {
				// the dir despawned before the command ran: the dropped guard
				// resolves through the sweep.
				return;
			};
			// off the async runtime: read the dir's scoped store, await the
			// content scan, then spawn the route children and flush so the
			// route-tree observers settle against the whole hierarchy.
			entity_mut.run_async_local(
				async move |dir: AsyncEntity| -> Result {
					// resolved together, inside the task where the whole tree is
					// built so the ancestor links are reliably present: the store
					// the files load from, and the component this dir's unsectioned
					// frontmatter keys declare.
					let (store, frontmatter_type) = dir
						.with_state::<(
							Query<&BlobStore>,
							AncestorQuery<&FrontmatterType>,
						), Result<(BlobStore, FrontmatterType)>>(
							move |entity, (stores, types)| {
								Ok((
									scoped_store(
										&stores, entity, "RoutesDir", &src,
									)?,
									types
										.get(entity)
										.cloned()
										.unwrap_or_default(),
								))
							},
						)
						.await??;
					let specs = Self::discover_routes(
						&store,
						&filter,
						&frontmatter_type.component,
					)
					.await?;
					dir.world()
						.with(move |world| {
							// every valid route still spawns, so one bad slug does
							// not take the site down with it; the failures are
							// reported together once the guard has resolved.
							let mut failures = Vec::new();
							let mut spawned = Vec::new();
							for spec in specs {
								let path = spec.store_path.clone();
								match Self::spawn_route_spec(
									world, entity, spec,
								) {
									Ok(route) => spawned.push(route),
									Err(err) => failures
										.push(format!("`{path}`: {err}")),
								}
							}
							Self::swap_routes(world, entity, &spawned);
							world.flush();
							// routes are spawned: resolve, draining the root's set so
							// the deferred `Ready` fires.
							guard.resolve(world);
							failures
						})
						.await
						.xmap(|failures| match failures.is_empty() {
							true => Ok(()),
							// reported, never fatal: the healthy siblings are
							// already serving, so the raise must not take the app
							// down the way an unqualified error's default
							// `Severity::Panic` would
							false => Err(bevyhow!(
								"{} discovered route(s) failed to spawn:\n{}",
								failures.len(),
								failures.join("\n")
							)
							.with_severity(Severity::Error)),
						})
				},
			);
		});
		Ok(())
	}

	/// Spawn one discovered content file as a [`BlobPage`] route child of `parent`,
	/// hoisting the components its root declared onto the route entity. Spawned
	/// [`RouteHidden`] for [`swap_routes`](Self::swap_routes) to unhide. A
	/// declaration that will not insert takes its route with it, so the page
	/// gets no route rather than serving with the declaration defaulted away.
	fn spawn_route_spec(
		world: &mut World,
		parent: Entity,
		spec: RouteSpec,
	) -> Result<Entity> {
		let store_path = spec.store_path.clone();
		let (declarations, route_path) = Self::resolve_spec(world, spec)?;
		let mut route_entity = world.spawn((
			ChildOf(parent),
			route::new(route_path.as_str(), BlobPage::new(store_path)),
			HttpMethod::Get,
			ExportStrategy::Static,
			// a discovered content file is a user-facing page, so it carries
			// `PageRoute` and appears in the nav, like its codegen blob equivalent.
			PageRoute,
			RouteHidden,
		));
		// scan-time page metadata, so navigation knows titles/order up front
		if let Err(err) = declarations.insert(&mut route_entity) {
			route_entity.despawn();
			return Err(err);
		}
		Ok(route_entity.id())
	}

	/// A discovered file's declarations with the filename defaults applied
	/// ([`PageMeta::declare_file_defaults`]), and the url it serves at once its
	/// `slug` has had its say.
	///
	/// Resolved here rather than in the scan because reflect-building the
	/// declarations needs the world's type registry, and the router reads
	/// [`PageMeta`] out of them first because a `slug` has the last word on the
	/// url, before the route entity it would live on exists.
	fn resolve_spec(
		world: &World,
		spec: RouteSpec,
	) -> Result<(RootDeclarations, RelPath)> {
		let mut declarations = spec.declarations?;
		PageMeta::declare_file_defaults(&mut declarations, &spec.store_path);
		let meta = declarations.get::<PageMeta>(
			&world
				.get_resource::<AppTypeRegistry>()
				.ok_or_else(|| {
					bevyhow!("route discovery requires an `AppTypeRegistry`")
				})?
				.read(),
		)?;
		let route_path = Self::route_path_for(&spec.store_path, meta.as_ref())?;
		Ok((declarations, route_path))
	}

	/// Re-read `route`'s root declarations through its `blob` and apply them
	/// ([`apply_refresh`](Self::apply_refresh)), parking a pending guard on
	/// the build root (or the route outside a build) so a settle waits on it.
	fn refresh_route(world: &mut World, route: Entity, blob: Blob) {
		let guard = TemplatePending::park_on(
			world,
			TemplateBuildRoot::resolve(world, route),
			PendingKind::Passive,
			format!("`{}` refresh", blob.path()),
		);
		let Ok(mut entity_mut) = world.get_entity_mut(route) else {
			return;
		};
		// local for the same reason the scan is: the bridge poll is only
		// guaranteed on the runtime's local executor.
		entity_mut.run_async_local(async move |route: AsyncEntity| -> Result {
			let frontmatter_type = route
				.with_state::<AncestorQuery<&FrontmatterType>, _>(
					|entity, types| {
						types.get(entity).cloned().unwrap_or_default()
					},
				)
				.await?;
			let spec = RouteSpec {
				declarations: Self::scan_declarations(
					blob.store(),
					blob.path(),
					&frontmatter_type.component,
				)
				.await,
				store_path: blob.path().clone(),
			};
			let entity = route.id();
			route
				.world()
				.with(move |world| -> Result {
					let outcome = Self::apply_refresh(world, entity, spec);
					world.flush();
					guard.resolve(world);
					outcome
				})
				.await
		});
	}

	/// Hoist a refreshed `spec` onto `route` in place when its url is
	/// unchanged, else rescan the owning [`RoutesDir`]: the swap is the one way
	/// a route's path changes. A route with no dir (a codegen blob route) has a
	/// fixed url and refreshes in place regardless.
	fn apply_refresh(
		world: &mut World,
		route: Entity,
		spec: RouteSpec,
	) -> Result {
		let (declarations, route_path) = Self::resolve_spec(world, spec)?;
		let moved = world.get::<PathPartial>(route).is_some_and(|current| {
			*current != PathPartial::new(route_path.as_str())
		});
		let dir = world.get::<ChildOf>(route).map(ChildOf::parent).and_then(
			|parent| {
				world
					.get::<RoutesDir>(parent)
					.cloned()
					.map(|dir| (parent, dir))
			},
		);
		match (moved, dir) {
			(true, Some((dir, rescan))) => {
				world.entity_mut(dir).insert(rescan);
				Ok(())
			}
			_ => declarations.insert(&mut world.entity_mut(route)),
		}
	}

	/// Swap `dir`'s routes for the ones this scan `spawned`: every route a
	/// previous scan left (a child carrying a [`PathPattern`] this scan did not
	/// spawn) is retired first, then the new set is unhidden, so at no flush do
	/// old and new paths coexist in the tree (a duplicate fails the rebuild).
	/// Runs in the scan's one world access, so no request observes the window
	/// between; a route already [`Retired`] by an earlier swap is left to its
	/// sweep.
	fn swap_routes(world: &mut World, dir: Entity, spawned: &[Entity]) {
		let previous = world
			.entity(dir)
			.get::<Children>()
			.map(|children| {
				children
					.iter()
					.filter(|child| !spawned.contains(child))
					.filter(|child| {
						let child = world.entity(*child);
						child.contains::<PathPattern>()
							&& !child.contains::<Retired>()
					})
					.collect::<Vec<_>>()
			})
			.unwrap_or_default();
		for route in previous {
			Retired::retire(world, route);
		}
		for route in spawned {
			world.entity_mut(*route).remove::<RouteHidden>();
		}
	}

	/// List the store's content files and read each one's declared metadata,
	/// returning route specs in lexical path order so zero-padded routes (eg slides
	/// `01..20`) spawn in sequence, giving a deterministic [`RouteTree`] child order.
	///
	/// This half is the store I/O; what the bytes MEAN settles at spawn time (see
	/// [`spawn_route_spec`](Self::spawn_route_spec)), which is also where a `slug`
	/// renames the route path — after the sort, so it cannot reshuffle the order.
	///
	/// Listing the dir is a hard error, but a file that will not scan rides its
	/// own spec and fails at spawn alongside a bad slug, so a half-typed
	/// frontmatter does not take the whole dir down on live reload.
	async fn discover_routes(
		store: &BlobStore,
		filter: &GlobFilter,
		frontmatter_type: &str,
	) -> Result<Vec<RouteSpec>> {
		let mut paths = store.list().await?;
		paths.sort();
		paths
			.into_iter()
			.filter(|path| Self::is_content(path) && filter.passes(path))
			.map(async |path| RouteSpec {
				declarations: Self::scan_declarations(
					store,
					&path,
					frontmatter_type,
				)
				.await,
				store_path: path,
			})
			.xmap(async_ext::join_all)
			.await
			.xok()
	}

	/// Whether `path`'s extension marks it as a servable content file.
	fn is_content(path: &RelPath) -> bool {
		path.extension()
			.is_some_and(|ext| CONTENT_EXTENSIONS.contains(&ext))
	}

	/// The route path a content file serves at before frontmatter has its say:
	/// the extension is dropped and a trailing `index` collapses to its
	/// directory, eg `docs/index.md` -> `docs`.
	///
	/// A pure function of the filename, shared with the codegen collection scan
	/// so a route path never depends on which scan found the file. The
	/// frontmatter `slug` override is applied on top by the caller, which holds
	/// the resolved [`PageMeta`] (see [`PageMeta::apply_slug`]).
	pub(crate) fn route_path_of(rel: &RelPath) -> RelPath {
		let mut segments = rel.segments();
		if let (Some(stem), Some(last)) = (rel.file_stem(), segments.last_mut())
		{
			*last = stem;
		}
		if segments.last() == Some(&"index") {
			segments.pop();
		}
		RelPath::from_segments(&segments)
	}

	/// The url a content file serves at: its filename-derived path
	/// ([`route_path_of`](Self::route_path_of)) with a declared `slug` renaming
	/// the final segment.
	///
	/// # Errors
	/// Errors when an `index` file declares a slug. Such a file collapses into
	/// its directory, so the segment a slug would rename belongs to the
	/// DIRECTORY, not the page: `blog/index.md` with `slug = "journal"` would
	/// serve at `/journal` while every sibling post stayed under `/blog`.
	fn route_path_for(
		store_path: &RelPath,
		meta: Option<&PageMeta>,
	) -> Result<RelPath> {
		let route_path = Self::route_path_of(store_path);
		let Some(meta) = meta.filter(|meta| meta.slug.is_some()) else {
			return Ok(route_path);
		};
		if store_path.file_stem() == Some("index") {
			bevybail!(
				"`{store_path}` declares a slug, but an index file collapses into its directory, \
				so it has no page segment of its own to rename"
			);
		}
		meta.apply_slug(&route_path)
	}

	/// Read a content file's ROOT declarations through the store: markdown
	/// frontmatter, or the root spreads of a BSX document. An extension with no
	/// declaration surface (`.html`) declares nothing; a file that cannot be read
	/// or parsed is an error, so a malformed page fails discovery loudly instead
	/// of silently serving with no title, order or slug.
	async fn scan_declarations(
		store: &BlobStore,
		path: &RelPath,
		frontmatter_type: &str,
	) -> Result<RootDeclarations> {
		let source = store.get(path).await?.to_vec().xmap(String::from_utf8)?;
		match path.extension() {
			Some("md" | "mdx" | "markdown") => Frontmatter::extract(&source)?
				.map(|frontmatter| frontmatter.declarations(frontmatter_type))
				.unwrap_or_default(),
			Some("bsx") => BsxNode::parse_document(&source, &default())?
				.xmap(|nodes| RootDeclarations::from_bsx(&nodes)),
			_ => default(),
		}
		.xok()
	}
}

/// A discovered content file: the store path its bytes load from, and the
/// components that file declares at its root — or the reason its root would not
/// read, carried per file so it is reported by name with the other spawn
/// failures rather than aborting the scan.
struct RouteSpec {
	store_path: RelPath,
	declarations: Result<RootDeclarations>,
}

/// Refresh each discovered route whose file changed: its root declarations are
/// read again and hoisted in place, the route entity and its url untouched, so
/// an edited title or order reaches the nav without a rescan and a request in
/// flight keeps its route. An edit that moves the url (a `slug`) rescans the
/// owning [`RoutesDir`] instead. A route the scan just spawned is skipped: that
/// scan read its declarations.
pub(crate) fn refresh_changed_routes(
	routes: Query<
		(Entity, Ref<Blob>),
		(Changed<Blob>, With<BlobPage>, Without<RouteHidden>),
	>,
	mut commands: Commands,
) {
	for (route, blob) in routes.iter().filter(|(_, blob)| !blob.is_added()) {
		let blob = Blob::clone(&blob);
		commands.queue(move |world: &mut World| {
			RoutesDir::refresh_route(world, route, blob)
		});
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;
	use beet_ui::prelude::*;

	fn router_world() -> World { (AsyncPlugin, RouterPlugin).into_world() }

	/// Compose `store` on the root (the repo store an entry carries) so the
	/// [`RoutesDir`] resolves it by ancestry, then settle the async runtime so the
	/// discovery task (an async store scan) completes. Mirrors a boot path settling
	/// before it serves.
	async fn spawn_routes(
		world: &mut World,
		store: impl Bundle,
		bundle: impl Bundle,
	) -> Entity {
		let root = world.spawn((store, bundle)).flush();
		AsyncRunner::settle_async_tasks(world).await;
		root
	}

	/// A router world with the main schedule, so the blob reactions run on
	/// [`react`].
	fn reactive_world() -> World {
		(MinimalPlugins, AsyncPlugin, RouterPlugin).into_world()
	}

	/// Run one frame (draining the store's events into the reactions) and settle
	/// the tasks they spawned.
	async fn react(world: &mut World) {
		world.update_local();
		AsyncRunner::settle_async_tasks(world).await;
	}

	/// An in-memory store seeded with `files`, as its concrete component (so
	/// spawning it subscribes its watcher) plus a handle for writing beside the
	/// world.
	async fn reactive_fixture(
		files: &[(&str, &str)],
	) -> (InMemoryStore, BlobStore) {
		let inner = InMemoryStore::new();
		let handle = BlobStore::new(inner.clone());
		for (rel, content) in files {
			handle
				.insert(&RelPath::from(*rel), content.to_string())
				.await
				.unwrap();
		}
		(inner, handle)
	}

	/// The title a route's hoisted [`PageMeta`] declares.
	fn title_of(world: &World, route: Entity) -> String {
		world.get::<PageMeta>(route).unwrap().title.clone().unwrap()
	}

	/// A byte change to a content file refreshes its route in place: the edited
	/// title reaches the route entity, which keeps its id, so nothing retires
	/// and a request in flight keeps its route.
	#[beet_core::test]
	async fn edit_refreshes_the_route_in_place() {
		let mut world = reactive_world();
		let (inner, handle) = reactive_fixture(&[(
			"post.md",
			"+++\ntitle = \"First\"\n+++\n\n# Post",
		)])
		.await;
		let root = spawn_routes(
			&mut world,
			inner,
			(Router, children![RoutesDir::default()]),
		)
		.await;
		let post = RouteTree::of(&world, root)
			.unwrap()
			.find(&["post"])
			.unwrap()
			.entity;
		title_of(&world, post).xpect_eq("First");

		handle
			.insert(
				&RelPath::from("post.md"),
				"+++\ntitle = \"Second\"\n+++\n\n# Post",
			)
			.await
			.unwrap();
		react(&mut world).await;
		title_of(&world, post).xpect_eq("Second");
		world.query_once::<&Retired>().len().xpect_eq(0);
	}

	/// A `slug` edit moves the url, which no in-place refresh can express: the
	/// owning dir rescans and swaps the route.
	#[beet_core::test]
	async fn slug_edit_rescans_the_dir() {
		let mut world = reactive_world();
		let (inner, handle) = reactive_fixture(&[(
			"post.md",
			"+++\nslug = \"first\"\n+++\n\n# Post",
		)])
		.await;
		let root = spawn_routes(
			&mut world,
			inner,
			(Router, children![RoutesDir::default()]),
		)
		.await;
		let first = RouteTree::of(&world, root)
			.unwrap()
			.find(&["first"])
			.unwrap()
			.entity;

		handle
			.insert(
				&RelPath::from("post.md"),
				"+++\nslug = \"second\"\n+++\n\n# Post",
			)
			.await
			.unwrap();
		react(&mut world).await;
		let tree = RouteTree::of(&world, root).unwrap().clone();
		tree.find(&["first"]).xpect_none();
		tree.find(&["second"]).xpect_some();
		world.get_entity(first).is_err().xpect_true();
	}

	/// A created file marks the dir's store changed, which rescans it.
	#[beet_core::test]
	async fn created_file_rescans_the_dir() {
		let mut world = reactive_world();
		let (inner, handle) = reactive_fixture(&[("index.md", "# Home")]).await;
		let root = spawn_routes(
			&mut world,
			inner,
			(Router, children![RoutesDir::default()]),
		)
		.await;
		handle
			.insert(&RelPath::from("about.md"), "# About")
			.await
			.unwrap();
		react(&mut world).await;
		RouteTree::of(&world, root)
			.unwrap()
			.find(&["about"])
			.xpect_some();
	}

	/// Write a routes dir fixture under `target/tests` and return a [`BlobStore`]
	/// backed by an [`FsStore`] rooted at it. Native-only: writes real files.
	#[cfg(not(target_arch = "wasm32"))]
	fn fs_fixture(name: &str, files: &[(&str, &str)]) -> BlobStore {
		let root = fs_ext::workspace_root()
			.join("target/tests/routes_dir")
			.join(name);
		// clean slate so removed fixture files do not leak between runs
		fs_ext::remove(&root).ok();
		for (rel, content) in files {
			fs_ext::write(root.join(rel), content).unwrap();
		}
		BlobStore::new(FsStore::new(AbsPath::new(root).unwrap()))
	}

	/// An in-memory [`BlobStore`] seeded with `files`, proving discovery is
	/// provider-agnostic (the same scan the S3-backed task runs).
	async fn memory_fixture(files: &[(&str, &str)]) -> BlobStore {
		let store = BlobStore::temp();
		for (rel, content) in files {
			store
				.insert(&RelPath::from(*rel), content.to_string())
				.await
				.unwrap();
		}
		store
	}

	#[beet_core::test]
	fn route_path_of() {
		RoutesDir::route_path_of(&RelPath::from("docs/intro.md"))
			.xpect_eq(RelPath::new("docs/intro"));
		RoutesDir::route_path_of(&RelPath::from("index.md"))
			.xpect_eq(RelPath::default());
		RoutesDir::route_path_of(&RelPath::from("docs/index.md"))
			.xpect_eq(RelPath::new("docs"));
		RoutesDir::route_path_of(&RelPath::from("about.bsx"))
			.xpect_eq(RelPath::new("about"));
	}

	/// Assert the three fixture routes render their content, shared by the
	/// filesystem- and memory-backed cases so both providers prove the same scan.
	async fn assert_serves(world: &mut World, root: Entity) {
		for (path, expected) in [
			("", "welcome"),
			("docs/intro", "the intro"),
			("about", "About"),
		] {
			world
				.entity_mut(root)
				.exchange(
					Request::get(path)
						.with_header::<header::Accept>(vec![MediaType::Html]),
				)
				.await
				.unwrap_str()
				.await
				.xpect_contains(expected);
		}
	}

	const SERVES_FILES: &[(&str, &str)] = &[
		("index.md", "# Home\n\nwelcome"),
		("docs/intro.md", "# Intro\n\nthe intro"),
		("about.bsx", "<main><h1>About</h1></main>"),
	];

	/// The filesystem-backed variant: discovery reads real files through an
	/// [`FsStore`]. Native-only (no real fs on wasm); the wasm path is covered by
	/// [`discovers_and_serves_from_memory_store`] over the same files.
	#[cfg(not(target_arch = "wasm32"))]
	#[beet_core::test]
	async fn discovers_and_serves_routes() {
		let mut world = router_world();
		let root = spawn_routes(
			&mut world,
			fs_fixture("serves", SERVES_FILES),
			(Router::with_defaults(), children![RoutesDir::default()]),
		)
		.await;
		assert_serves(&mut world, root).await;
	}

	/// A dir's derived [`DirPath`] scopes its store and registers its
	/// [`WatchDir`], the one registration site every mount shares.
	#[cfg(not(target_arch = "wasm32"))]
	#[beet_core::test]
	async fn scopes_and_watches_its_dir() {
		let mut world = router_world();
		let store = fs_fixture("watches", SERVES_FILES);
		let root = spawn_routes(
			&mut world,
			store.clone(),
			(Router, children![RoutesDir::new("docs")]),
		)
		.await;
		let dir = world.entity(root).get::<Children>().unwrap()[0];
		let scoped = store.with_subdir(RelPath::new("docs"));
		world
			.entity(dir)
			.get::<BlobStore>()
			.unwrap()
			.same_scope(&scoped)
			.xpect_true();
		world
			.entity(dir)
			.get::<WatchDir>()
			.unwrap()
			.dir
			.xpect_eq(scoped.watch_dir().unwrap());
	}

	/// The same site loads identically from a non-filesystem store: discovery,
	/// route paths and content reads all go through the [`BlobStore`] abstraction.
	#[beet_core::test]
	async fn discovers_and_serves_from_memory_store() {
		let mut world = router_world();
		let root = spawn_routes(
			&mut world,
			memory_fixture(SERVES_FILES).await,
			(Router::with_defaults(), children![RoutesDir::default()]),
		)
		.await;
		assert_serves(&mut world, root).await;
	}

	/// Discovered files are sorted lexically before spawning, so the [`RouteTree`]
	/// children come out in filename order regardless of store list order. Store
	/// agnostic, so it runs over the in-memory store and covers wasm too.
	#[beet_core::test]
	async fn routes_spawn_in_sorted_order() {
		let mut world = router_world();
		// a bare `Router` (not `Router::with_defaults`) so the opinionated app routes do
		// not appear as extra top-level children alongside the discovered slides.
		// deliberately out-of-order, zero-padded like the slide deck.
		let root = spawn_routes(
			&mut world,
			memory_fixture(&[
				("03-gamma.md", "# Gamma"),
				("01-alpha.md", "# Alpha"),
				("02-beta.md", "# Beta"),
			])
			.await,
			(Router, children![RoutesDir::default()]),
		)
		.await;

		let tree = world.entity(root).get::<RouteTree>().unwrap().clone();
		// the discovered slide routes, in tree-child order
		tree.children
			.iter()
			.filter_map(|child| child.path.iter().last())
			.map(|seg| seg.name().to_string())
			.collect::<Vec<_>>()
			.xpect_eq(vec!["01-alpha", "02-beta", "03-gamma"]);
	}

	/// Re-inserting a [`RoutesDir`] rescans it as a swap: the tree serves
	/// exactly the rescanned set, every route it holds is a fresh, unhidden
	/// entity, and the routes the first scan spawned are gone rather than
	/// lingering beside them.
	#[beet_core::test]
	async fn rescan_swaps_the_routes() {
		let mut world = router_world();
		let store = memory_fixture(&[("index.md", "# Home")]).await;
		let root = spawn_routes(
			&mut world,
			store.clone(),
			(Router, children![RoutesDir::default()]),
		)
		.await;
		let dir = world.entity(root).get::<Children>().unwrap()[0];
		let first = RouteTree::of(&world, root)
			.unwrap()
			.find(&[] as &[&str])
			.unwrap()
			.entity;

		store
			.insert(&RelPath::from("about.md"), "# About")
			.await
			.unwrap();
		world.entity_mut(dir).insert(RoutesDir::default());
		AsyncRunner::settle_async_tasks(&mut world).await;

		let tree = RouteTree::of(&world, root).unwrap().clone();
		let home = tree.find(&[] as &[&str]).unwrap().entity;
		home.xpect_not_eq(first);
		world.get_entity(first).is_err().xpect_true();
		tree.find(&["about"]).xpect_some();
		// the dir holds the rescanned set and nothing else, none of it hidden
		let routes = world.entity(dir).get::<Children>().unwrap();
		routes.len().xpect_eq(2);
		routes
			.iter()
			.any(|route| world.entity(route).contains::<RouteHidden>())
			.xpect_false();
	}

	/// A numbered file declaring a `slug` serves at the slug, not at its
	/// filename, and keeps the filename's number as its nav order — the pair
	/// that lets a directory read in order while its urls stay stable names.
	#[beet_core::test]
	async fn slug_overrides_the_filename_path() {
		let mut world = router_world();
		let root = spawn_routes(
			&mut world,
			memory_fixture(&[(
				"blog/1-full-stack-bevy.md",
				"+++\nslug = \"full-stack-bevy\"\n+++\n\n# Post",
			)])
			.await,
			(Router, children![RoutesDir::default()]),
		)
		.await;

		let tree = world.entity(root).get::<RouteTree>().unwrap().clone();
		tree.find(&["blog", "1-full-stack-bevy"]).xpect_none();
		let node = tree.find(&["blog", "full-stack-bevy"]).unwrap();
		world
			.entity(node.entity)
			.get::<PageMeta>()
			.unwrap()
			.order
			.unwrap()
			.xpect_eq(1);
	}

	/// A slug renames the page's own segment, and an index file — which has none,
	/// having collapsed into its directory — is told so rather than quietly
	/// renaming the directory out from under its siblings.
	#[beet_core::test]
	fn route_path_for_applies_slug() {
		let slugged = PageMeta {
			slug: Some("full-stack-bevy".into()),
			..default()
		};
		RoutesDir::route_path_for(
			&RelPath::from("blog/1-full-stack-bevy.md"),
			Some(&slugged),
		)
		.unwrap()
		.xpect_eq(RelPath::new("blog/full-stack-bevy"));
		// no slug declared, the filename stands
		RoutesDir::route_path_for(&RelPath::from("blog/index.bsx"), None)
			.unwrap()
			.xpect_eq(RelPath::new("blog"));
		RoutesDir::route_path_for(
			&RelPath::from("blog/index.bsx"),
			Some(&slugged),
		)
		.unwrap_err()
		.to_string()
		.xpect_contains("no page segment of its own");
	}

	/// A filtered root dir and a scoped dir under a `<Route>` compose to the urls
	/// one unfiltered dir would serve: the excluded subtree is discovered by the
	/// second dir instead, which is what lets it carry its own layout.
	#[beet_core::test]
	async fn filter_splits_a_dir_across_two_scans() {
		let mut world = router_world();
		let files = &[
			("index.md", "# Home"),
			("docs/intro.md", "# Intro"),
			("blog/index.md", "# Blog"),
			("blog/1-post.md", "# Post"),
		];
		let root = spawn_routes(
			&mut world,
			memory_fixture(files).await,
			(Router, children![
				RoutesDir::default()
					.with_filter(GlobFilter::default().with_exclude("blog/**")),
				(PathPartial::new("blog"), children![RoutesDir::new("blog")])
			]),
		)
		.await;

		let tree = world.entity(root).get::<RouteTree>().unwrap().clone();
		// the blog subtree serves at the same urls the single-dir scan gave it
		tree.find(&["blog"]).xpect_some();
		tree.find(&["blog", "1-post"]).xpect_some();
		tree.find(&["docs", "intro"]).xpect_some();
		// ..and the root dir discovered it once, not twice (a duplicate route
		// would have failed the tree build outright)
		tree.find(&["blog", "blog"]).xpect_none();
	}

	/// A BSX page declares the same metadata markdown puts in frontmatter, as the
	/// component itself on its root; the scan reads it without building the
	/// document, so the route carries it before anyone visits the page.
	#[beet_core::test]
	async fn scan_time_bsx_spread_meta() {
		let mut world = router_world();
		let root = spawn_routes(
			&mut world,
			memory_fixture(&[(
				"blog/index.bsx",
				r#"<Fragment {PageMeta{title: "The Full Moon Harvest", created: "2025-09-06", sidebar_label: "Blog", order: 1}}><h1>Blog</h1></Fragment>"#,
			)])
			.await,
			(Router, children![RoutesDir::default()]),
		)
		.await;

		let tree = world.entity(root).get::<RouteTree>().unwrap().clone();
		let node = tree.find(&["blog"]).unwrap();
		let meta = world.entity(node.entity).get::<PageMeta>().unwrap();
		meta.title
			.as_deref()
			.unwrap()
			.xpect_eq("The Full Moon Harvest");
		meta.sidebar_label.as_deref().unwrap().xpect_eq("Blog");
		meta.order.unwrap().xpect_eq(1);
		// the date string coerces to the instant it names
		meta.created
			.unwrap()
			.format_long_date()
			.xpect_eq("6 September 2025");
	}

	/// The scan hoists WHATEVER a document declares, not one blessed type: a
	/// TOML `[Section]` names its component by short type path and lands beside
	/// the default component's keys.
	#[beet_core::test]
	async fn hoists_sectioned_components() {
		let mut world = router_world();
		let root = spawn_routes(
			&mut world,
			memory_fixture(&[(
				"blog/post.md",
				"+++\ntitle = \"Post\"\n[Layout]\ntemplate = \"ArticleLayout\"\n+++\n\n# Post",
			)])
			.await,
			(Router, children![RoutesDir::default()]),
		)
		.await;

		let tree = world.entity(root).get::<RouteTree>().unwrap().clone();
		let entity = tree.find(&["blog", "post"]).unwrap().entity;
		world
			.entity(entity)
			.get::<PageMeta>()
			.unwrap()
			.title
			.as_deref()
			.unwrap()
			.xpect_eq("Post");
		world
			.entity(entity)
			.get::<Layout>()
			.unwrap()
			.template
			.as_str()
			.xpect_eq("ArticleLayout");
	}

	/// A dir declaring its own [`FrontmatterType`] redirects the unsectioned
	/// keys, so a site's own metadata component needs no change to the scan.
	#[beet_core::test]
	async fn frontmatter_type_overrides_the_default_component() {
		let mut world = router_world();
		let root = spawn_routes(
			&mut world,
			memory_fixture(&[(
				"post.md",
				"+++\ntemplate = \"ArticleLayout\"\n+++\n\n# Post",
			)])
			.await,
			(Router, children![(RoutesDir::default(), FrontmatterType {
				component: "Layout".into()
			})]),
		)
		.await;

		let tree = world.entity(root).get::<RouteTree>().unwrap().clone();
		let entity = tree.find(&["post"]).unwrap().entity;
		world
			.entity(entity)
			.get::<Layout>()
			.unwrap()
			.template
			.as_str()
			.xpect_eq("ArticleLayout");
		world
			.entity(entity)
			.get::<PageMeta>()
			.is_none()
			.xpect_true();
	}

	/// A section naming a component this binary does not register warns and is
	/// skipped: the page still serves, carrying every declaration that did
	/// resolve, exactly as an unregistered tag still builds its subtree.
	#[beet_core::test]
	async fn unknown_section_does_not_take_the_page_down() {
		let mut world = router_world();
		let root = spawn_routes(
			&mut world,
			memory_fixture(&[(
				"post.md",
				"+++\ntitle = \"Post\"\n[NotInThisBinary]\nfoo = \"bar\"\n+++\n\n# Post",
			)])
			.await,
			(Router, children![RoutesDir::default()]),
		)
		.await;

		let tree = world.entity(root).get::<RouteTree>().unwrap().clone();
		let entity = tree.find(&["post"]).unwrap().entity;
		world
			.entity(entity)
			.get::<PageMeta>()
			.unwrap()
			.title
			.as_deref()
			.unwrap()
			.xpect_eq("Post");
	}

	/// A declaration that will not resolve fails its own file: the page gets no
	/// route rather than silently serving with the field defaulted away, and its
	/// siblings still spawn, the same resilience a bad slug gets.
	#[beet_core::test]
	async fn unresolvable_declaration_does_not_take_the_dir_down() {
		let mut world = router_world();
		let root = spawn_routes(
			&mut world,
			memory_fixture(&[
				("good.md", "+++\norder = 1\n+++\n\n# Good"),
				("bad_meta.md", "+++\norder = \"not-a-number\"\n+++\n\n# Bad"),
				("bad_markup.bsx", "<main {Unterminated"),
			])
			.await,
			(Router, children![RoutesDir::default()]),
		)
		.await;

		let tree = world.entity(root).get::<RouteTree>().unwrap().clone();
		tree.find(&["good"]).is_some().xpect_true();
		tree.find(&["bad_meta"]).is_none().xpect_true();
		tree.find(&["bad_markup"]).is_none().xpect_true();
	}

	/// Frontmatter is scanned from file content through the store, so it is store
	/// agnostic and runs over the in-memory store (covering wasm too).
	#[beet_core::test]
	async fn scan_time_frontmatter_meta() {
		let mut world = router_world();
		let root = spawn_routes(
			&mut world,
			memory_fixture(&[(
				"docs/intro.md",
				"+++\ntitle = \"Getting Started\"\norder = 2\n+++\n\n# Intro",
			)])
			.await,
			(Router::with_defaults(), children![RoutesDir::default()]),
		)
		.await;

		let tree = world.entity(root).get::<RouteTree>().unwrap().clone();
		let node = tree.find(&["docs", "intro"]).unwrap().clone();
		let meta = world.entity(node.entity).get::<PageMeta>().unwrap();
		meta.title.as_deref().unwrap().xpect_eq("Getting Started");
		meta.order.unwrap().xpect_eq(2);
	}
}
