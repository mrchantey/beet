//! The route driver behind `beet check`, `export-static` and the `--watch` dev
//! serve: render every static route's content into a persistent tree, run
//! [`render_diagnostics`] over it, and aggregate the results.
//!
//! Each scene route's own `Action<Request, PageRequest>` builds its content
//! through the template substrate, so the built tree — incl any [`TemplateError`]
//! the build rode — is there to scan. Cleanup then mirrors [`PageRoot::render`]:
//! only the route's [`DespawnAfterRender`] ephemerals are despawned, never the
//! `content` entity, which for a `BlobScene`/`RoutesDir` route is the persistent
//! [`RouteTree`] node that every later request reuses.

use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_ui::prelude::RuleSet;

/// The outcome of a [`check_routes`] pass: every [`Diagnostic`] collected across
/// the site's routes, with convenience accessors for the gated entry points.
#[derive(Debug, Clone, Default)]
pub struct CheckReport {
	/// Every diagnostic found, route context attached.
	pub diagnostics: Vec<Diagnostic>,
	/// The static route paths that were rendered and scanned.
	pub checked: Vec<SmolPath>,
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

/// Run [`check_routes`] over every router in the world and log each
/// [`Diagnostic`] loudly, the dev-serve surfacing path: after a build (or a
/// `--watch` reload) every render problem prints to the console at its severity.
///
/// A best-effort console pass: a router that fails to scan is logged and skipped
/// rather than aborting, so a transient build error never kills the dev loop.
/// Returns whether any error-level diagnostic fired across all routers.
pub async fn log_all_render_diagnostics(world: &AsyncWorld) -> bool {
	let routers = world
		.with(|world: &mut World| {
			world
				.query_filtered::<Entity, With<RouteTree>>()
				.iter(world)
				.collect::<Vec<_>>()
		})
		.await;
	let mut had_error = false;
	for router in routers {
		match check_routes(world, router).await {
			Ok(report) => {
				report.log();
				had_error |= report.has_errors();
			}
			Err(error) => error!("render-diagnostics scan failed: {error}"),
		}
	}
	had_error
}

/// Render every static route under `router`, run [`render_diagnostics`] over each
/// built content tree, and return the aggregated [`CheckReport`].
///
/// A route is checked when its path is fully static and its method is `GET`
/// (mirroring `export-static`), so the scan covers exactly the pages a no-code
/// site ships. The [`RenderDiagnostics`] config is read from the world (defaulting
/// when absent); the [`RuleSet`] is re-read per route *after* its build, so a
/// `bx:style`/`inline_class!` rule a route registers at build time is matched
/// rather than flagged unknown.
pub async fn check_routes(
	world: &AsyncWorld,
	router: Entity,
) -> Result<CheckReport> {
	// the static GET routes worth checking, plus the route tree + config snapshot
	// every per-route scan validates against.
	let (route_entities, route_tree, config) = world
		.with(move |world: &mut World| -> Result<_> {
			let route_tree = world
				.entity(router)
				.get::<RouteTree>()
				.ok_or_else(|| {
					bevyhow!("router entity {router} has no RouteTree")
				})?
				.clone();
			let config = world
				.get_resource::<RenderDiagnostics>()
				.cloned()
				.unwrap_or_default();
			let route_entities = route_tree
				.flatten_nodes()
				.into_iter()
				.filter(|node| checkable(node))
				.map(|node| (node.entity, node.path.annotated_path()))
				.collect::<Vec<_>>();
			Ok((route_entities, route_tree, config))
		})
		.await?;

	let mut report = CheckReport::default();
	for (entity, path) in route_entities {
		check_route(world, entity, &path, &route_tree, &config, &mut report)
			.await?;
		report.checked.push(path);
	}
	Ok(report)
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

/// Build one route's content into a persistent tree, scan it, then despawn it.
///
/// The route's own `Action<Request, PageRequest>` builds the content without
/// despawning (unlike the full render), so the tree — with any build
/// [`TemplateError`] — is present to walk. A build failure that surfaces as an
/// `Err` (rather than riding `TemplateError`) folds in as an unknown-tag error so
/// it is never silently dropped.
async fn check_route(
	world: &AsyncWorld,
	entity: Entity,
	path: &SmolPath,
	route_tree: &RouteTree,
	config: &RenderDiagnostics,
	report: &mut CheckReport,
) -> Result {
	let request = Request::get(path.with_leading_slash());
	let built = world
		.entity(entity)
		.call::<Request, PageRequest>(request)
		.await;
	match built {
		Ok(PageRequest(content)) => {
			// the `world.with` closure must be `'static`, so own the snapshots the
			// scan reads (cloned once per route, not per element).
			let (route_tree, config) = (route_tree.clone(), config.clone());
			let route = path.clone();
			let diagnostics = world
				.with(move |world: &mut World| {
					// re-read the rule set *after* the build, so a `bx:style`/inline
					// rule this route registered is present and not flagged unknown.
					let rule_set = world
						.get_resource::<RuleSet>()
						.cloned()
						.unwrap_or_default();
					let out = render_diagnostics(
						world,
						content,
						&route_tree,
						&rule_set,
						&config,
					)
					.into_iter()
					.map(|diagnostic| diagnostic.with_route(route.clone()))
					.collect::<Vec<_>>();
					// clean up exactly what a real render would: the route's
					// `DespawnAfterRender` ephemerals (a per-request route's whole
					// tree, a scene route's parsed children), never `content` itself.
					// For a `BlobScene`/`RoutesDir` route `content` is the *persistent*
					// route-tree node, so despawning it leaves every `RouteTree` entry
					// dangling and 500s the next serve/export.
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
					out
				})
				.await;
			report.diagnostics.extend(diagnostics);
		}
		// a build that bailed with an `Err` (not riding `TemplateError`) is still a
		// loud, route-attached unknown-tag error rather than a silent skip.
		Err(error) => {
			let severity = config.severity(DiagnosticKind::UnknownTag);
			if severity != DiagnosticSeverity::Off {
				report.diagnostics.push(
					Diagnostic::new(
						DiagnosticKind::UnknownTag,
						severity,
						format!("failed to build route: {error}"),
					)
					.with_route(path.clone()),
				);
			}
		}
	}
	Ok(())
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

	/// Run [`check_routes`] over `router` and return the report.
	async fn check(world: &mut World, router: Entity) -> CheckReport {
		world
			.run_async_then(async move |world| {
				check_routes(&world, router).await
			})
			.await
			.unwrap()
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

	/// A persistent scene route (a `BlobScene`, as `RoutesDir` spawns) survives a
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
			.spawn((store, Router, children![route(
				"post",
				BlobScene::new("post.md")
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
			.call::<Request, Response>(
				Request::get("post").with_accept(MediaType::Html),
			)
			.await
			.unwrap()
			.unwrap_str()
			.await
			.xpect_contains("body");
	}
}
