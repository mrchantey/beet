//! The route driver behind `beet check`, `export-static` and the `--watch` dev
//! serve: render every static route's content into a persistent tree, run
//! [`render_diagnostics`] over it, and aggregate the results.
//!
//! Each scene route's own `Action<Request, PageRequest>` builds its content
//! through the template substrate, so the built tree, incl any [`TemplateError`]
//! the build rode, is there to scan. Cleanup then mirrors [`PageRoot::render`]:
//! only the route's [`DespawnAfterRender`] ephemerals are despawned, never the
//! `content` entity, which for a `BlobPage`/`RoutesDir` route is the persistent
//! [`RouteTree`] node that every later request reuses.

use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_ui::prelude::RuleSet;
use std::sync::Arc;

/// The outcome of a [`CheckReport::check_routes`] pass: every [`Diagnostic`] collected across
/// the site's routes, with convenience accessors for the gated entry points.
#[derive(Debug, Clone, Default)]
pub struct CheckReport {
	/// Every diagnostic found, route context attached.
	pub diagnostics: Vec<Diagnostic>,
	/// The static route paths that were rendered and scanned.
	pub checked: Vec<RelPath>,
}

impl CheckReport {
	/// Whether any [`Error`](DiagnosticSeverity::Error)-level diagnostic fired, ie whether a
	/// gated entry point should exit non-zero.
	pub fn has_errors(&self) -> bool {
		self.diagnostics.iter().any(Diagnostic::is_error)
	}

	/// The number of error-level diagnostics.
	pub fn error_count(&self) -> usize {
		self.diagnostics.iter().filter(|d| d.is_error()).count()
	}

	/// The number of warning-level diagnostics.
	pub fn warn_count(&self) -> usize {
		self.diagnostics
			.iter()
			.filter(|d| d.severity == DiagnosticSeverity::Warn)
			.count()
	}

	/// Log every diagnostic through the `log` facade at its severity level.
	pub fn log(&self) {
		for diagnostic in &self.diagnostics {
			diagnostic.log();
		}
	}
}

/// Run [`CheckReport::check_routes_in`] over every router in the world and
/// log each [`Diagnostic`] loudly, the dev-serve surfacing path: after a build
/// (or a `--watch` reload) every render problem prints to the console at its
/// severity. `routes` narrows the pass as [`CheckReport::check_routes_in`] does.
///
/// A best-effort console pass: a router that fails to scan is logged and skipped
/// rather than aborting, so a transient build error never kills the dev loop.
///
/// Rides `client_io`'s native-only gate: the live reload loop is its only caller.
#[cfg(all(feature = "client_io", not(target_arch = "wasm32")))]
pub(crate) async fn log_all_render_diagnostics(
	world: &AsyncWorld,
	routes: Option<HashSet<Entity>>,
) {
	let routers = world
		.with(|world: &mut World| {
			world
				.query_filtered::<Entity, With<RouteTree>>()
				.iter(world)
				.collect::<Vec<_>>()
		})
		.await;
	for router in routers {
		match CheckReport::check_routes_in(world, router, routes.clone()).await
		{
			Ok(report) => report.log(),
			Err(error) => error!("render-diagnostics scan failed: {error}"),
		}
	}
}

impl CheckReport {
	/// Render every static route under `root`, run `render_diagnostics` over each
	/// built content tree, and return the aggregated [`CheckReport`].
	///
	/// `root` is whatever the caller holds: `beet check` and the exports pass the
	/// loaded entry root, the dev serve one router at a time. [`RouteTree::of`]
	/// resolves the tree from either.
	///
	/// A route is checked when its path is fully static and its method is `GET`
	/// (mirroring `export-static`), so the scan covers exactly the pages a no-code
	/// site ships. The [`RenderDiagnostics`] config is read from the world (defaulting
	/// when absent); the [`RuleSet`] is re-read per route *after* its build, so a
	/// `bx:style`/`inline_class!` rule a route registers at build time is matched
	/// rather than flagged unknown.
	///
	/// The pass opens on `root`'s own structure: an unresolvable tag loads as an
	/// inert [`UnregisteredTag`] entity in every binary, so this, run by a
	/// binary that registers everything, is where the typo it might be surfaces
	/// as an error.
	pub async fn check_routes(
		world: &AsyncWorld,
		root: Entity,
	) -> Result<CheckReport> {
		Self::check_routes_in(world, root, None).await
	}

	/// [`Self::check_routes`] narrowed to the `routes` entities, every static
	/// route when `None`: a content reload knows which files changed, so a
	/// markdown edit re-checks its one page rather than the whole site.
	///
	/// Each route is checked by its own world task (see `check_route`), so a
	/// caller cancelled mid-pass (a reload tail superseded by the next reload)
	/// never orphans a build: the route in flight finishes and cleans up on its
	/// own, and the cancellation lands between routes.
	pub async fn check_routes_in(
		world: &AsyncWorld,
		root: Entity,
		routes: Option<HashSet<Entity>>,
	) -> Result<CheckReport> {
		// the static GET routes worth checking, plus the route tree + config snapshot
		// every per-route scan validates against, and the document's own inert tags.
		let (route_entities, route_tree, config, unregistered) = world
			.with(move |world: &mut World| -> Result<_> {
				let route_tree = RouteTree::of(world, root)?.clone();
				let config = world
					.get_resource::<RenderDiagnostics>()
					.cloned()
					.unwrap_or_default();
				let route_entities = route_tree
					.flatten_nodes()
					.into_iter()
					.filter(|node| {
						checkable(node)
							&& routes.as_ref().is_none_or(|routes| {
								routes.contains(&node.entity)
							})
					})
					.map(|node| (node.entity, node.path.annotated_path()))
					.collect::<Vec<_>>();
				let unregistered = unregistered_tags(world, root, &config);
				Ok((route_entities, Arc::new(route_tree), config, unregistered))
			})
			.await?;

		let mut report = CheckReport::default();
		report.diagnostics.extend(unregistered);
		for (entity, path) in route_entities {
			let diagnostics = check_route(
				world,
				entity,
				path.clone(),
				route_tree.clone(),
				config.clone(),
			)
			.await;
			report.diagnostics.extend(diagnostics);
			report.checked.push(path);
		}
		// a persistent route's content is reachable from both the document scan
		// and its own route scan; drop the routeless duplicate.
		let routed = report
			.diagnostics
			.iter()
			.filter(|diagnostic| diagnostic.route.is_some())
			.map(|diagnostic| diagnostic.message.clone())
			.collect::<HashSet<_>>();
		report.diagnostics.retain(|diagnostic| {
			diagnostic.route.is_some() || !routed.contains(&diagnostic.message)
		});
		Ok(report)
	}
}

/// Every [`UnregisteredTag`] in `root`'s subtree, as an
/// [`UnknownTag`](DiagnosticKind::UnknownTag) diagnostic.
///
/// Route *content* is scanned per route by [`render_diagnostics`] once it is
/// built; this covers the document's own structure, which is present from load.
fn unregistered_tags(
	world: &mut World,
	root: Entity,
	config: &RenderDiagnostics,
) -> Vec<Diagnostic> {
	let severity = config.severity(DiagnosticKind::UnknownTag);
	if severity == DiagnosticSeverity::Off {
		return Vec::new();
	}
	world.with_state::<(Query<&UnregisteredTag>, Query<&Children>), _>(
		|(tags, children)| {
			children
				.iter_descendants_inclusive(root)
				.filter_map(|entity| tags.get(entity).ok())
				.map(|tag| Diagnostic::unregistered_tag(tag, severity))
				.collect()
		},
	)
}

/// Whether a route node is a static `GET` page worth scanning: a fully-static
/// path whose method is `GET` (or unset), and which builds a render tree (a scene
/// route). Mirrors `export-static`'s selection.
fn checkable(node: &ActionNode) -> bool {
	node.path.is_static()
		&& node
			.method
			.map(|method| method == HttpMethod::Get)
			.unwrap_or(true)
		&& node.is_scene()
}

/// Build one route's content into a persistent tree, scan it, then despawn its
/// ephemerals, returning the route's diagnostics.
///
/// Runs as a world task, cancellation-proof by ownership rather than by a
/// guard: the build is the route entity's own action task and completes
/// whatever becomes of this caller, and its [`DespawnAfterRender`] ephemerals
/// are only known once it resolves, so a caller cancelled mid-build (a reload
/// tail superseded by the next reload) would orphan them and no drop guard
/// could release them. The world task always reaches its cleanup instead and
/// hands the diagnostics back over a oneshot: a cancelled caller drops the
/// receiver, the route finishes and cleans up on its own, and its result goes
/// nowhere.
async fn check_route(
	world: &AsyncWorld,
	entity: Entity,
	path: RelPath,
	route_tree: Arc<RouteTree>,
	config: RenderDiagnostics,
) -> Vec<Diagnostic> {
	let (send, recv) = OnceValue::oneshot();
	world
		.run_async(move |world| async move {
			send.signal(
				build_and_scan(&world, entity, path, route_tree, config).await,
			);
		})
		.await;
	recv.wait().await
}

/// The body of [`check_route`]. The route's own `Action<Request, PageRequest>`
/// builds the content without despawning (unlike the full render), so the
/// tree, with any build [`TemplateError`], is present to walk; the scan and the
/// ephemeral cleanup share one world access. A build failure that surfaces as
/// an `Err` (rather than riding `TemplateError`) folds in as an unknown-tag
/// error so it is never silently dropped.
async fn build_and_scan(
	world: &AsyncWorld,
	entity: Entity,
	path: RelPath,
	route_tree: Arc<RouteTree>,
	config: RenderDiagnostics,
) -> Vec<Diagnostic> {
	let request = Request::get(path.with_leading_slash());
	match world
		.entity(entity)
		.call::<Request, PageRequest>(request)
		.await
	{
		Ok(PageRequest(content)) => {
			world
				.with(move |world: &mut World| {
					// re-read the rule set *after* the build, so a `bx:style`/inline
					// rule this route registered is present and not flagged unknown.
					let rule_set = world
						.get_resource::<RuleSet>()
						.cloned()
						.unwrap_or_default();
					let diagnostics = render_diagnostics(
						world,
						content,
						&route_tree,
						&rule_set,
						&config,
					)
					.into_iter()
					.map(|diagnostic| diagnostic.with_route(path.clone()))
					.collect::<Vec<_>>();
					despawn_ephemerals(world, content);
					diagnostics
				})
				.await
		}
		// a build that bailed with an `Err` (not riding `TemplateError`) is still a
		// loud, route-attached unknown-tag error rather than a silent skip.
		Err(error) => match config.severity(DiagnosticKind::UnknownTag) {
			DiagnosticSeverity::Off => Vec::new(),
			severity => vec![
				Diagnostic::new(
					DiagnosticKind::UnknownTag,
					severity,
					format!("failed to build route: {error}"),
				)
				.with_route(path),
			],
		},
	}
}

/// Clean up exactly what a real render would: the route's
/// [`DespawnAfterRender`] ephemerals (a per-request route's whole tree, a scene
/// route's parsed children), never `content` itself. For a
/// `BlobPage`/`RoutesDir` route `content` is the *persistent* route-tree node,
/// so despawning it leaves every [`RouteTree`] entry dangling and 500s the next
/// serve/export.
fn despawn_ephemerals(world: &mut World, content: Entity) {
	let ephemerals = world
		.get_entity(content)
		.ok()
		.and_then(|entity| {
			entity
				.get::<DespawnAfterRender>()
				.map(|despawn| despawn.0.clone())
		})
		.unwrap_or_default();
	for entity in ephemerals {
		if let Ok(entity) = world.get_entity_mut(entity) {
			entity.despawn();
		}
	}
}

#[cfg(test)]
mod test {
	use super::*;
	// the parent imports only `beet_ui::prelude::RuleSet`; the tests also build
	// real rules and the material rule set.
	use beet_ui::prelude::*;

	/// A router world whose `RuleSet` carries a `.page` rule.
	fn check_world() -> World {
		let mut world = (
			AsyncPlugin,
			RouterPlugin,
			material::MaterialStylePlugin::default(),
		)
			.into_world();
		world
			.get_resource_or_init::<RuleSet>()
			.insert_rule(Rule::class("page"));
		world
	}

	/// Run [`CheckReport::check_routes`] over `router` and return the report.
	async fn check(world: &mut World, router: Entity) -> CheckReport {
		world
			.run_async_then(async move |world| {
				CheckReport::check_routes(&world, router).await
			})
			.await
			.unwrap()
	}

	/// The number of render ephemerals still alive, zero once every checked
	/// route has been cleaned up.
	fn ephemerals(world: &mut World) -> usize {
		world
			.query_filtered::<(), With<DespawnAfterRender>>()
			.iter(world)
			.count()
	}

	#[beet_core::test]
	async fn clean_site_has_no_errors() {
		let mut world = check_world();
		let router = world
			.spawn((Router, children![
				render_action::fixed_func_route("", || {
					rsx! { <div class="page"><a href="/about">"about"</a></div> }
				}),
				render_action::fixed_func_route("about", || {
					rsx! { <p>"about"</p> }
				}),
			]))
			.flush();
		let report = check(&mut world, router).await;
		report.has_errors().xpect_false();
		// both static scene routes were scanned.
		report.checked.len().xpect_eq(2);
		ephemerals(&mut world).xpect_eq(0);
	}

	/// A narrowed pass checks only its routes.
	#[beet_core::test]
	async fn narrows_to_the_given_routes() {
		let mut world = check_world();
		let router = world
			.spawn((Router, children![
				render_action::fixed_func_route("", || rsx! { <p>"home"</p> }),
				render_action::fixed_func_route("about", || {
					rsx! { <p>"about"</p> }
				}),
			]))
			.flush();
		let about = world
			.entity(router)
			.get::<RouteTree>()
			.unwrap()
			.find(&["about"])
			.unwrap()
			.entity;
		world
			.run_async_then(async move |world| {
				CheckReport::check_routes_in(
					&world,
					router,
					Some([about].into_iter().collect()),
				)
				.await
			})
			.await
			.unwrap()
			.checked
			.xpect_eq(vec![RelPath::from("about")]);
	}

	/// A caller cancelled while a route check is in flight (the reload tail
	/// superseded by the next reload) leaves nothing behind: the build finishes
	/// on the route, its ephemerals despawn, and its diagnostics reach nobody.
	#[beet_core::test]
	async fn cancelled_mid_route_leaves_no_ephemerals() {
		let mut world = check_world();
		// a route whose build parks on a gate the test opens after the cancel,
		// logging each side of it
		let (open, gate) = OnceValue::<()>::oneshot();
		let gate = Store::new(Some(gate));
		let log = Store::<Vec<&'static str>>::default();
		let router = world
			.spawn((Router, children![render_action::async_route(
				"",
				move |_cx: ActionContext<Request>| async move {
					log.push("parked");
					if let Some(gate) = gate.take() {
						gate.wait().await;
					}
					log.push("built");
					rsx! { <a href="/does-not-exist">"x"</a> }
				}
			)]))
			.flush();
		let report = Store::<Option<CheckReport>>::default();
		let task = world.run_task(move |world| async move {
			let out = CheckReport::check_routes(&world, router).await.unwrap();
			report.set(Some(out));
		});
		// drive until the pass is parked on the route's build, then cancel it
		// and open the gate
		for _ in 0..100 {
			if log.get() == vec!["parked"] {
				break;
			}
			world.update_local();
			AsyncRunner::tick(&world).await;
		}
		log.get().xpect_eq(vec!["parked"]);
		task.cancel();
		open.signal(());
		AsyncRunner::settle_async_tasks(&mut world).await;
		// the build ran to completion, cleaned up, and reported nothing
		log.get().xpect_eq(vec!["parked", "built"]);
		report.get().xpect_none();
		ephemerals(&mut world).xpect_eq(0);
		world.resource::<AsyncSpawner>().in_flight().xpect_eq(0);
	}

	#[beet_core::test]
	async fn broken_href_fails() {
		let mut world = check_world();
		let router = world
			.spawn((Router, children![render_action::fixed_func_route(
				"",
				|| rsx! { <a href="/does-not-exist">"x"</a> }
			)]))
			.flush();
		let report = check(&mut world, router).await;
		report.has_errors().xpect_true();
		report
			.diagnostics
			.iter()
			.any(|d| d.kind == DiagnosticKind::BrokenHref && d.is_error())
			.xpect_true();
	}

	#[beet_core::test]
	async fn unknown_class_warns_only() {
		let mut world = check_world();
		let router = world
			.spawn((Router, children![render_action::fixed_func_route(
				"",
				|| rsx! { <div class="zzz-not-real"/> }
			)]))
			.flush();
		let report = check(&mut world, router).await;
		// a lone unknown class warns but does not fail.
		report.has_errors().xpect_false();
		report.warn_count().xpect_eq(1);
	}

	/// An unresolvable tag loads as an inert entity in every binary, so this pass,
	/// run by a binary that registers everything, is where it surfaces as the
	/// typo it is, even though it sits in the document rather than in any route's
	/// rendered content.
	#[beet_core::test]
	async fn unregistered_tag_errors() {
		let mut world = check_world();
		let router = world
			.spawn((Router, children![
				render_action::fixed_func_route("", || rsx! { <p>"home"</p> }),
				UnregisteredTag::new("Butonn"),
			]))
			.flush();
		let report = check(&mut world, router).await;
		report.has_errors().xpect_true();
		report
			.diagnostics
			.iter()
			.any(|diagnostic| {
				diagnostic.kind == DiagnosticKind::UnknownTag
					&& diagnostic.message.contains("Butonn")
			})
			.xpect_true();
	}

	/// A persistent scene route (a `BlobPage`, as `RoutesDir` spawns) survives a
	/// `check_routes` pass and still renders afterwards. The route's `PageRequest`
	/// content entity *is* its persistent `RouteTree` node, so the scan must despawn
	/// only its parsed children, never the node itself. Regression for a boot-time
	/// check pass that despawned every route node, 500-ing every later serve request
	/// and `export-static` with "entity despawned, generation N".
	#[beet_core::test]
	async fn scene_route_survives_check_and_renders() {
		let store = BlobStore::temp();
		store
			.insert(&"post.md".into(), "# Title\n\nbody".to_owned())
			.await
			.unwrap();
		let mut world = check_world();
		let router = world
			.spawn((store, Router, children![route::new(
				"post",
				BlobPage::new("post.md")
			)]))
			.flush();
		// the boot-time diagnostics pass, the despawn hazard.
		check(&mut world, router).await.has_errors().xpect_false();
		// the route node entity is still alive in the tree (not despawned).
		let node = world
			.entity(router)
			.get::<RouteTree>()
			.unwrap()
			.find(&["post"])
			.unwrap()
			.clone();
		world.get_entity(node.entity).is_ok().xpect_true();
		// and it still renders, the symptom a real serve/export would hit.
		world
			.entity_mut(router)
			.exchange(Request::get("post").with_accept(MediaType::Html))
			.await
			.unwrap_str()
			.await
			.xpect_contains("body");
	}
}
