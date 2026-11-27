//! The route driver behind `beet check`, `export-static` and the `--watch` dev
//! serve: render every static route's content into a persistent tree, run
//! [`render_diagnostics`] over it and check its [`DocMeta`] against the
//! [`DocMetaRules`], and aggregate the results, then, for the
//! whole-document pass, check every derived file the document declares.
//!
//! Each scene route's own `Action<Request, PageRequest>` builds its content
//! through the template substrate, so the built tree, incl any [`TemplateError`]
//! the build rode, is there to scan. Cleanup then mirrors [`LivePage::respond`]:
//! only the route's [`DespawnAfterRender`] ephemerals are despawned, never the
//! `content` entity, which for a `BlobPage`/`RoutesDir` route is the persistent
//! [`RouteTree`] node that every later request reuses.

use super::render_diagnostics::DiagnosticsQuery;
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_scene::prelude::*;
use beet_ui::prelude::DocMeta;
use beet_ui::prelude::DocMetaRules;
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
	/// The pass opens on `root`'s own structure: an unresolvable tag or spread
	/// loads held in an [`Unregistered`] in every binary, so this, run by a
	/// binary that registers everything, is where the typo it might be surfaces
	/// as an error. It closes on the derived files the document declares, each
	/// missing or stale one a diagnostic, which a narrowed pass skips since no
	/// page edit changes them.
	pub async fn check_routes(
		world: &AsyncWorld,
		root: Entity,
	) -> Result<CheckReport> {
		let mut report = Self::check_routes_in(world, root, None).await?;
		report
			.diagnostics
			.extend(derived_file_diagnostics(world, root).await?);
		Ok(report)
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
		// every per-route scan validates against, the document's own inert tags
		// and the discovered files that failed to become routes.
		let (route_entities, route_tree, config, unserved) = world
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
				let mut unserved = unregistered_tags(world, root, &config);
				unserved.extend(failed_routes(world, root, &config));
				Ok((route_entities, Arc::new(route_tree), config, unserved))
			})
			.await?;

		let mut report = CheckReport::default();
		report.diagnostics.extend(unserved);
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

/// Every component held [`Unregistered`] in `root`'s subtree, as an
/// [`Unregistered`](DiagnosticKind::Unregistered) diagnostic.
///
/// Route *content* is scanned per route by [`render_diagnostics`] once it is
/// built; this covers the document's own structure, which is present from load.
fn unregistered_tags(
	world: &mut World,
	root: Entity,
	config: &RenderDiagnostics,
) -> Vec<Diagnostic> {
	let severity = config.severity(DiagnosticKind::Unregistered);
	if severity == DiagnosticSeverity::Off {
		return Vec::new();
	}
	world.with_state::<DiagnosticsQuery, _>(|query| {
		query.unregistered(root, severity)
	})
}

/// Every file a [`RoutesDir`] in `root`'s subtree failed to serve
/// ([`FailedRoutes`]), as a [`FailedRoute`](DiagnosticKind::FailedRoute)
/// diagnostic: the page is simply absent, so without this it surfaces only as
/// the broken links pointing at it.
fn failed_routes(
	world: &mut World,
	root: Entity,
	config: &RenderDiagnostics,
) -> Vec<Diagnostic> {
	let severity = config.severity(DiagnosticKind::FailedRoute);
	if severity == DiagnosticSeverity::Off {
		return Vec::new();
	}
	world.with_state::<(Query<&FailedRoutes>, Query<&Children>), _>(
		|(failed, children)| {
			children
				.iter_descendants_inclusive(root)
				.filter_map(|entity| failed.get(entity).ok())
				.flat_map(|failed| failed.iter())
				.map(|(path, reason)| {
					Diagnostic::new(
						DiagnosticKind::FailedRoute,
						severity,
						format!("`{path}` could not be served: {reason}"),
					)
				})
				.collect()
		},
	)
}

/// A [`MissingDerivedFile`](DiagnosticKind::MissingDerivedFile),
/// [`UnwrittenDerivedFile`](DiagnosticKind::UnwrittenDerivedFile) or
/// [`StaleDerivedFile`](DiagnosticKind::StaleDerivedFile) diagnostic for each
/// derived file declared under `root` that is not fresh in the store it
/// resolves by ancestry, naming the verb that writes one when it declares it.
async fn derived_file_diagnostics(
	world: &AsyncWorld,
	root: Entity,
) -> Result<Vec<Diagnostic>> {
	let (config, derivations) = world
		.with(move |world: &mut World| -> Result<_> {
			let config = world
				.get_resource::<RenderDiagnostics>()
				.cloned()
				.unwrap_or_default();
			let derivations = Derivation::declared(world, root)
				.into_iter()
				.map(|entity| {
					world
						.with_state::<(Query<&Derivation>, AncestorQuery<&BlobStore>), _>(
							|(derivations, stores)| {
								(
									entity,
									derivations.get(entity)?.clone(),
									stores.get(entity)?.clone(),
								)
									.xok()
							},
						)
				})
				.collect::<Result<Vec<_>>>()?;
			(config, derivations).xok()
		})
		.await?;
	let mut diagnostics = Vec::new();
	for (entity, derivation, store) in derivations {
		let path = &derivation.path;
		let freshness = Derivation::fresh(world, entity, &store).await?;
		let (kind, message) = match (freshness, &derivation.written_by) {
			(Freshness::Fresh, _) => continue,
			(Freshness::Missing, Some(verb)) => (
				DiagnosticKind::UnwrittenDerivedFile,
				format!(
					"derived file `{path}` is not written yet: `{verb}` writes \
					 it, and the site serves without it until then"
				),
			),
			(Freshness::Missing, None) => (
				DiagnosticKind::MissingDerivedFile,
				format!(
					"derived file `{path}` is missing: the site serves without it"
				),
			),
			// an incomplete set serves broken links, so it fails as missing
			(Freshness::OutputMissing(output), _) => (
				DiagnosticKind::MissingDerivedFile,
				format!(
					"derived file `{path}` is incomplete, output `{output}` \
					 is missing: the site serves without it"
				),
			),
			(stale, Some(verb)) => (
				DiagnosticKind::StaleDerivedFile,
				format!(
					"derived file `{path}` is stale: {stale}, `{verb}` refreshes it"
				),
			),
			(stale, None) => (
				DiagnosticKind::StaleDerivedFile,
				format!("derived file `{path}` is stale: {stale}"),
			),
		};
		match config.severity(kind) {
			DiagnosticSeverity::Off => {}
			severity => {
				diagnostics.push(Diagnostic::new(kind, severity, message))
			}
		}
	}
	diagnostics.xok()
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
					.chain(doc_meta_diagnostics(world, content, &config))
					.map(|diagnostic| diagnostic.with_route(path.clone()))
					.collect::<Vec<_>>();
					despawn_ephemerals(world, content);
					diagnostics
				})
				.await
		}
		// a build that bailed with an `Err` (not riding `TemplateError`) is still a
		// loud, route-attached unregistered error rather than a silent skip.
		Err(error) => match config.severity(DiagnosticKind::Unregistered) {
			DiagnosticSeverity::Off => Vec::new(),
			severity => vec![
				Diagnostic::new(
					DiagnosticKind::Unregistered,
					severity,
					format!("failed to build route: {error}"),
				)
				.with_route(path),
			],
		},
	}
}

/// A [`DocMeta`](DiagnosticKind::DocMeta) diagnostic for each way the
/// `content` root's [`DocMeta`] breaks the world's [`DocMetaRules`].
fn doc_meta_diagnostics(
	world: &World,
	content: Entity,
	config: &RenderDiagnostics,
) -> Vec<Diagnostic> {
	let severity = config.severity(DiagnosticKind::DocMeta);
	let Some(meta) = world.get::<DocMeta>(content) else {
		return Vec::new();
	};
	if severity == DiagnosticSeverity::Off {
		return Vec::new();
	}
	let position = world
		.get::<SceneSource>(content)
		.and_then(|source| source.position(content));
	meta.validate(
		&world
			.get_resource::<DocMetaRules>()
			.cloned()
			.unwrap_or_default(),
	)
	.into_iter()
	.map(|violation| {
		Diagnostic::new(
			DiagnosticKind::DocMeta,
			severity,
			format!("page metadata: {violation}"),
		)
		.with_position(position.clone())
	})
	.collect()
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

	/// An unresolvable tag or spread loads held on its entity in every binary,
	/// so this pass, run by a binary that registers everything, is where each
	/// surfaces as the typo it is, even though it sits in the document rather
	/// than in any route's rendered content.
	#[beet_core::test]
	async fn unregistered_errors() {
		let mut world = check_world();
		let router = world.spawn(Router).id();
		ParseContext::new(
			&mut world.entity_mut(router),
			&MediaBytes::new_bsx(
				r#"<Butonn label="go"/><div {Mispelt{size: 2}}/>"#,
			),
		)
		.parse()
		.unwrap();
		world.flush();
		let report = check(&mut world, router).await;
		report.has_errors().xpect_true();
		for name in ["Butonn", "Mispelt"] {
			report
				.diagnostics
				.iter()
				.any(|diagnostic| {
					diagnostic.kind == DiagnosticKind::Unregistered
						&& diagnostic.message.contains(name)
				})
				.xpect_true();
		}
	}

	/// A scene body naming a component this binary never registered spawns its
	/// entity without it and holds it, which this pass elevates to an error
	/// exactly as it does a tag markup wrote.
	#[cfg(all(feature = "json", feature = "scene"))]
	#[beet_core::test]
	async fn an_unregistered_scene_component_errors() {
		let mut world = check_world();
		let router = world.spawn(Router).id();
		TemplateLoader::new(&mut world)
			.with_entity(router)
			.load(&MediaBytes::new_json(
				r#"{"$type":"org.beet.scene#body","entities":{"0":{"some_plugin::Sparkle":{"amount":3}}}}"#,
			))
			.unwrap();
		let report = check(&mut world, router).await;
		report.has_errors().xpect_true();
		report
			.diagnostics
			.iter()
			.any(|diagnostic| {
				diagnostic.kind == DiagnosticKind::Unregistered
					&& diagnostic.message.contains("some_plugin::Sparkle")
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

	/// A diagnostic says where the entity it is of was written, in a page's
	/// prose, in markup embedded in it and in a document it instantiates.
	#[cfg(feature = "markdown")]
	#[beet_core::test]
	async fn a_diagnostic_names_its_place() {
		let store = BlobStore::temp();
		for (path, source) in [
			(
				"post.md",
				"+++\ntitle = \"Post\"\n+++\n\n# Title\n\n[gone](/gone)\n\n<div class=\"nope\"><Typo/></div>\n\n<Instance document=\"/Card\"/>",
			),
			(
				"Card.bsx",
				"<section>\n\t<a href=\"/lost\">x</a>\n</section>",
			),
		] {
			store
				.insert(&RelPath::new(path), source.to_string())
				.await
				.unwrap();
		}
		let mut world = check_world();
		let router = world
			.spawn((
				PackageFiles::new(store.clone()),
				store,
				Router,
				children![route::new("post", BlobPage::new("post.md"))],
			))
			.flush();
		let mut places = check(&mut world, router)
			.await
			.diagnostics
			.into_iter()
			.map(|diagnostic| {
				format!(
					"{} {}",
					diagnostic.position.unwrap_or_default(),
					diagnostic.message.split(':').next().unwrap()
				)
			})
			.collect::<Vec<_>>();
		places.sort();
		places.xpect_eq(vec![
			"Card.bsx:2:2 broken internal link `href=\"/lost\"`".to_string(),
			"post.md:7:1 broken internal link `href=\"/gone\"`".into(),
			"post.md:9:1 unknown class `nope`".into(),
			"post.md:9:19 unregistered `Typo`".into(),
		]);
	}

	/// A page that does not parse says where.
	#[cfg(feature = "markdown")]
	#[beet_core::test]
	async fn a_malformed_page_names_its_place() {
		let store = BlobStore::temp();
		store
			.insert(
				&"post.md".into(),
				"# Title\n\n<p>\n<b bx:nope=\"x\"></b>\n</p>".to_owned(),
			)
			.await
			.unwrap();
		let mut world = check_world();
		let router = world
			.spawn((store, Router, children![route::new(
				"post",
				BlobPage::new("post.md")
			)]))
			.flush();
		check(&mut world, router).await.diagnostics[0]
			.message
			.xref()
			.xpect_contains("post.md:4:1: `bx:nope`");
	}

	/// A page whose metadata breaks the rules fails the check, one
	/// diagnostic per violation, and the vocabulary is the site's to declare.
	#[beet_core::test]
	async fn doc_meta_violations_fail() {
		let store = BlobStore::temp();
		store
			.insert(
				&"post.md".into(),
				"+++\ntags = [\"ecs\", \"Web\"]\n+++\n# Title\n\nbody"
					.to_owned(),
			)
			.await
			.unwrap();
		let mut world = check_world();
		world.insert_resource(DocMetaRules {
			tags: vec!["ecs".into()],
			..default()
		});
		let router = world
			.spawn((store, Router, children![route::new(
				"post",
				BlobPage::new("post.md")
			)]))
			.flush();
		let report = check(&mut world, router).await;
		report.has_errors().xpect_true();
		report
			.diagnostics
			.iter()
			.filter(|diagnostic| diagnostic.kind == DiagnosticKind::DocMeta)
			.map(|diagnostic| diagnostic.message.as_str())
			.collect::<Vec<_>>()
			.xpect_eq(vec![
				"page metadata: tag `Web` is not lowercase kebab, ie `local-first`",
			]);
	}

	/// A router over `store` declaring `derivation` of `derived/notes.json`
	/// from `notes.md`.
	fn derived_notes(
		world: &mut World,
		store: &BlobStore,
		derivation: Derivation,
	) -> Entity {
		world
			.spawn((store.clone(), Router, children![
				render_action::fixed_func_route("", || rsx! { <p>"home"</p> }),
				(
					derivation,
					Action::<BlobStore, EntityBody>::new_async(
						async |cx: ActionContext<BlobStore>| {
							let bytes =
								cx.input.get(&"notes.md".into()).await?;
							EntityBody::default().with_typed(&Provenance::new(
								[ProvenanceSource::file("notes.md", &bytes)],
							))
						}
					)
				)
			]))
			.flush()
	}

	/// The derived file diagnostics `report` carries, by kind and severity.
	fn derived(
		report: CheckReport,
	) -> Vec<(DiagnosticKind, DiagnosticSeverity)> {
		report
			.diagnostics
			.into_iter()
			.filter(|diagnostic| {
				matches!(
					diagnostic.kind,
					DiagnosticKind::MissingDerivedFile
						| DiagnosticKind::StaleDerivedFile
						| DiagnosticKind::UnwrittenDerivedFile
				)
			})
			.map(|diagnostic| (diagnostic.kind, diagnostic.severity))
			.collect()
	}

	/// A declared derived file is an error while missing, silent once fresh,
	/// and a warning once a source changes, since a stale one still serves.
	#[beet_core::test]
	async fn reports_missing_and_stale_derived_files() {
		let store = BlobStore::temp();
		store.insert(&"notes.md".into(), "first").await.unwrap();
		let mut world = check_world();
		let router = derived_notes(
			&mut world,
			&store,
			Derivation::new("derived/notes.json"),
		);
		check(&mut world, router)
			.await
			.xmap(derived)
			.xpect_eq(vec![(
				DiagnosticKind::MissingDerivedFile,
				DiagnosticSeverity::Error,
			)]);
		let derivation = Derivation::declared(&mut world, router)[0];
		let ensured = store.clone();
		world
			.run_async_then(async move |world| {
				Derivation::ensure(&world, derivation, &ensured).await
			})
			.await
			.unwrap();
		check(&mut world, router)
			.await
			.xmap(derived)
			.is_empty()
			.xpect_true();
		store.insert(&"notes.md".into(), "second").await.unwrap();
		check(&mut world, router)
			.await
			.xmap(derived)
			.xpect_eq(vec![(
				DiagnosticKind::StaleDerivedFile,
				DiagnosticSeverity::Warn,
			)]);
	}

	/// A derived file only its verb writes is a warning while missing, naming
	/// the verb, since the verb not having run yet is no fault.
	#[beet_core::test]
	async fn names_the_verb_of_an_unwritten_derived_file() {
		let store = BlobStore::temp();
		let mut world = check_world();
		let router = derived_notes(
			&mut world,
			&store,
			Derivation::new("derived/notes.json").with_written_by("publish"),
		);
		let report = check(&mut world, router).await;
		report
			.diagnostics
			.iter()
			.find(|diagnostic| {
				diagnostic.kind == DiagnosticKind::UnwrittenDerivedFile
			})
			.unwrap()
			.message
			.as_str()
			.xpect_contains("`publish` writes it");
		report.xmap(derived).xpect_eq(vec![(
			DiagnosticKind::UnwrittenDerivedFile,
			DiagnosticSeverity::Warn,
		)]);
	}
}
