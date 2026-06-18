//! The diagnostics manifest: a machine-readable dump of everything a future
//! editor (an LSP, a no-code authoring UI) would need to validate and
//! autocomplete a BSX site, without rendering it.
//!
//! [`beet check`](crate::prelude::check_routes) recovers the "type-checked
//! feeling" *reactively* — it scans a built tree and reports what is wrong. The
//! manifest is the *proactive* companion: it answers "what may I write here?"
//! ahead of time. It is the export side only; nothing here consumes it. Four
//! sections, each the source of truth a different completion would read:
//!
//! - [`tags`](DiagnosticsManifest::tags): every uppercase tag a BSX author can
//!   write (`<Header>`, `<Rule>`, `<SiteLayout>`, ..), with its prop schema where
//!   one is registered — a `<Tag …>` widget catalog + prop completion;
//! - [`classes`](DiagnosticsManifest::classes): every class with a style rule, the
//!   `class="…"` vocabulary (the *same* [`RuleSet`] scan the unknown-class check
//!   reads, so a manifest class is never one the check rejects);
//! - [`routes`](DiagnosticsManifest::routes): every route path, the `href="/…"`
//!   vocabulary (the *same* [`RouteTree`] the broken-href check validates against);
//! - [`style_props`](DiagnosticsManifest::style_props): the kebab property names a
//!   `<Rule>`/`bx:style` accepts.

use super::render_diagnostics::rule_set_classes;
use crate::prelude::*;
use beet_core::prelude::*;
use beet_ui::prelude::*;

/// A machine-readable description of what a BSX site exposes to its author: the
/// tags, classes, routes and style props a no-code editor would validate and
/// autocomplete against. Built by [`build_diagnostics_manifest`] and serialized to
/// JSON; nothing in beet reads it back (the consumer is a future editor).
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct DiagnosticsManifest {
	/// Every registered uppercase tag a BSX author can write, name + prop schema.
	pub tags: Vec<TagManifest>,
	/// Every class name carrying a style rule, the `class="..."` vocabulary.
	pub classes: Vec<SmolStr>,
	/// Every route path, the `href="/.."` vocabulary.
	pub routes: Vec<SmolStr>,
	/// The kebab property names a `<Rule>`/`bx:style` declaration accepts.
	pub style_props: Vec<SmolStr>,
}

/// One tag in the [`DiagnosticsManifest`] catalog: its author-facing name and,
/// where a schema is registered, the props it accepts and their types.
///
/// A tag with no registered schema (a bare `#[reflect(Component)]`/resource, or a
/// handler tag like `<Rule>`) is listed name-only — a future editor still offers
/// it, just without prop completion.
#[derive(Debug, Clone, serde::Serialize)]
pub struct TagManifest {
	/// The tag name as written in markup, eg `Header` or `path::to::Widget`.
	pub name: SmolStr,
	/// How the tag resolves, so a consumer can group widgets vs effects.
	pub kind: TagKind,
	/// The tag's prop schema, when one is registered (a `#[template]` signature or
	/// a `bx:schema` block). `None` when the tag is listed name-only.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub schema: Option<ValueSchema>,
}

/// How a manifest [`TagManifest`] resolves at build time, mirroring the BSX
/// resolver's resolution order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum TagKind {
	/// A `#[template]` (or `.bsx` file): builds an entity subtree.
	Template,
	/// A `#[reflect(Component)]` inserted onto the element.
	Component,
	/// A `#[reflect(Resource)]` patched at build time, rendering nothing.
	Resource,
	/// A [`BsxTagResolvers`] handler tag (eg `Rule`), a build-time effect.
	Handler,
}

/// Build the [`DiagnosticsManifest`] for the site rooted at `router`, gathering
/// from the loaded world exactly as a future editor would: the registries for
/// tags + schemas, the live [`RuleSet`] for classes, `router`'s [`RouteTree`] for
/// routes, and [`prop_name_map`] for style props.
///
/// Mirrors [`check_routes`](crate::prelude::check_routes)'s site access — same
/// `RouteTree` and `RuleSet` sources — so the manifest agrees with the checks.
pub fn build_diagnostics_manifest(
	world: &mut World,
	router: Entity,
) -> Result<DiagnosticsManifest> {
	Ok(DiagnosticsManifest {
		tags: manifest_tags(world),
		classes: manifest_classes(world),
		routes: manifest_routes(world, router)?,
		style_props: manifest_style_props(),
	})
}

/// Every uppercase tag a BSX author can write, sorted by name: registered Rust
/// types (template/component/resource via [`AppTypeRegistry`]), `.bsx` templates
/// ([`BsxTemplateRegistry`]) and handler tags ([`BsxTagResolvers`]), each with its
/// schema where one is registered. Names are de-duplicated, type-registry-first.
fn manifest_tags(world: &mut World) -> Vec<TagManifest> {
	let mut tags = HashMap::<SmolStr, TagManifest>::default();
	collect_type_registry_tags(world, &mut tags);
	collect_bsx_template_tags(world, &mut tags);
	collect_handler_tags(world, &mut tags);
	let mut tags = tags.into_values().collect::<Vec<_>>();
	tags.sort_by(|left, right| left.name.cmp(&right.name));
	tags
}

/// Collect every registered Rust type that resolves as an uppercase tag: a
/// `#[template]` ([`ReflectTemplate`]), a resource or a component. The schema
/// comes from the type's [`ReflectTemplateSchema`] when present (only templates
/// carry one), so a component/resource is listed name-only.
fn collect_type_registry_tags(
	world: &World,
	tags: &mut HashMap<SmolStr, TagManifest>,
) {
	use bevy::ecs::reflect::ReflectComponent;
	use bevy::ecs::reflect::ReflectResource;
	let Some(registry) = world.get_resource::<AppTypeRegistry>() else {
		return;
	};
	let registry = registry.read();
	for registration in registry.iter() {
		let name = registration.type_info().type_path_table().short_path();
		// a tag a BSX author can write resolves by name and is uppercase; lowercase
		// types (plain data) are never markup tags.
		if !is_uppercase_tag(name) {
			continue;
		}
		let kind = if registration.data::<ReflectTemplate>().is_some() {
			TagKind::Template
		} else if registration.data::<ReflectResource>().is_some() {
			TagKind::Resource
		} else if registration.data::<ReflectComponent>().is_some() {
			TagKind::Component
		} else {
			// not insertable/buildable from markup, so not an author-facing tag.
			continue;
		};
		// only a template carries a prop schema (its `ReflectTemplateSchema`); a
		// wildcard schema validates nothing, so it is dropped to "name-only".
		let schema = registration
			.data::<ReflectTemplateSchema>()
			.map(|data| data.schema())
			.filter(|schema| !matches!(schema, ValueSchema::Any))
			.cloned();
		tags.insert(SmolStr::from(name), TagManifest {
			name: SmolStr::from(name),
			kind,
			schema,
		});
	}
}

/// Collect every `.bsx`-authored template ([`BsxTemplateRegistry`]) by its module
/// path, with the prop schema from its `bx:schema` block when declared.
fn collect_bsx_template_tags(
	world: &World,
	tags: &mut HashMap<SmolStr, TagManifest>,
) {
	let Some(registry) = world.get_resource::<BsxTemplateRegistry>() else {
		return;
	};
	for (name, schema) in registry.manifest() {
		tags.entry(name.clone()).or_insert_with(|| TagManifest {
			name: name.clone(),
			kind: TagKind::Template,
			schema: schema.cloned(),
		});
	}
}

/// Collect every [`BsxTagResolvers`] handler tag (eg `Rule`), a build-time effect
/// with no prop schema.
fn collect_handler_tags(
	world: &World,
	tags: &mut HashMap<SmolStr, TagManifest>,
) {
	let Some(resolvers) = world.get_resource::<BsxTagResolvers>() else {
		return;
	};
	for name in resolvers.keys() {
		tags.entry(name.clone()).or_insert_with(|| TagManifest {
			name: name.clone(),
			kind: TagKind::Handler,
			schema: None,
		});
	}
}

/// Every class name with a style rule, de-duplicated and sorted. Reuses A5's
/// [`rule_set_classes`] scan, so the catalog and the unknown-class check share one
/// source of truth.
fn manifest_classes(world: &World) -> Vec<SmolStr> {
	let Some(rule_set) = world.get_resource::<RuleSet>() else {
		return Vec::new();
	};
	let mut classes = rule_set_classes(rule_set)
		.map(SmolStr::from)
		.collect::<Vec<_>>();
	classes.sort();
	classes.dedup();
	classes
}

/// Every route path under `router`, rooted with a leading slash and sorted.
/// Reuses the [`RouteTree`] the broken-href check validates against.
fn manifest_routes(world: &World, router: Entity) -> Result<Vec<SmolStr>> {
	let route_tree = world
		.entity(router)
		.get::<RouteTree>()
		.ok_or_else(|| bevyhow!("router entity {router} has no RouteTree"))?;
	let mut routes = route_tree
		.flatten()
		.iter()
		.map(|pattern| {
			SmolStr::from(pattern.annotated_path().with_leading_slash())
		})
		.collect::<Vec<_>>();
	routes.sort();
	routes.dedup();
	Ok(routes)
}

/// The kebab property names a `<Rule>`/`bx:style` declaration accepts, from A3's
/// [`prop_name_map`](beet_ui::prelude::style::prop_name_map), sorted.
fn manifest_style_props() -> Vec<SmolStr> {
	let mut props = style::prop_name_map().into_keys().collect::<Vec<_>>();
	props.sort();
	props
}

/// Whether a tag resolves by name (uppercase) rather than as an HTML element,
/// shared with the render-diagnostics literal-tag check.
fn is_uppercase_tag(tag: &str) -> bool {
	tag.rsplit("::")
		.next()
		.unwrap_or(tag)
		.starts_with(|ch: char| ch.is_uppercase())
}

#[cfg(test)]
mod test {
	use super::*;

	/// A loaded-site world: the router substrate plus the Material style plugin —
	/// which transitively registers the widget tag set (`Header`, ..) and the
	/// `<Rule>` handler — and a couple of routes, the four manifest sources in
	/// miniature.
	fn manifest_world() -> (World, Entity) {
		let mut world = (
			AsyncPlugin,
			RouterPlugin,
			// the Material rule set + (via `BsxDefaultsPlugin`/`CssPlugin`) the widget
			// tags and the `<Rule>` handler tag.
			material::MaterialStylePlugin::default(),
		)
			.into_world();
		world
			.get_resource_or_init::<RuleSet>()
			.insert_rule(Rule::class("page"));
		let router = world
			.spawn((Router, children![
				render_action::fixed_func_route("", || rsx! { <p>"home"</p> }),
				render_action::fixed_func_route("about", || {
					rsx! { <p>"about"</p> }
				}),
			]))
			.flush();
		(world, router)
	}

	#[beet_core::test]
	fn manifest_lists_every_source() {
		let (mut world, router) = manifest_world();
		let manifest = build_diagnostics_manifest(&mut world, router).unwrap();

		// routes: both static scene routes, rooted.
		manifest.routes.contains(&SmolStr::from("/")).xpect_true();
		manifest
			.routes
			.contains(&SmolStr::from("/about"))
			.xpect_true();

		// classes: the `.page` rule + Material classes, agreeing with the check.
		manifest
			.classes
			.contains(&SmolStr::from("page"))
			.xpect_true();

		// tags: a known widget (`Header`) and the `<Rule>` handler.
		let tag =
			|name: &str| manifest.tags.iter().find(|tag| tag.name == name);
		tag("Header").xpect_some();
		(tag("Rule").unwrap().kind == TagKind::Handler).xpect_true();

		// style props: A3's kebab vocabulary.
		manifest
			.style_props
			.contains(&SmolStr::from("display"))
			.xpect_true();
	}

	#[beet_core::test]
	fn tag_schema_is_wired() {
		let (mut world, router) = manifest_world();
		let manifest = build_diagnostics_manifest(&mut world, router).unwrap();
		// at least one registered template tag carries a real prop schema (the
		// schema machinery is wired, not always-`None`).
		manifest
			.tags
			.iter()
			.any(|tag| {
				tag.kind == TagKind::Template
					&& matches!(tag.schema, Some(ValueSchema::Struct(_)))
			})
			.xpect_true();
	}

	#[beet_core::test]
	fn round_trips_to_valid_json() {
		let (mut world, router) = manifest_world();
		let manifest = build_diagnostics_manifest(&mut world, router).unwrap();
		let json = serde_json::to_string_pretty(&manifest).unwrap();
		// re-parsing proves it is valid JSON carrying the expected sections.
		let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
		parsed["routes"].is_array().xpect_true();
		parsed["classes"].is_array().xpect_true();
		parsed["tags"].is_array().xpect_true();
		parsed["style_props"].is_array().xpect_true();
		// a known route survives the JSON round-trip.
		json.as_str().xpect_contains("/about");
	}
}
