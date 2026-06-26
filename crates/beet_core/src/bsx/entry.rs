//! Booting an app's entity tree from a single `.bsx` entry document.
//!
//! [`BsxTemplate::parse_entry`] parses `main.bsx`-style source as an entry
//! document: exactly one root element, which [`BsxTemplate::spawn`] builds
//! *into* a fresh entity, so root-level components (eg a `Router`) land on the
//! returned entity exactly as `world.spawn((Router, ..))` would place them. This
//! differs from the document-parse convention ([`BsxTemplate::container`]) where
//! every root node spawns as a child of a container. [`BsxTemplate::load_entry`]
//! is the file convenience over it.

use super::*;
use crate::prelude::*;
#[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
use std::path::Path;

impl BsxTemplate {
	/// Parse `source` as an entry document and return the template for its single
	/// root element, ready to build into a root entity.
	///
	/// Like an XML document element, exactly one root element is required;
	/// comments and whitespace at the root are ignored. `<path::to::X>` tags
	/// resolve against the world's current [`BsxTemplateRegistry`] (snapshotted
	/// here), so register any template directories (eg via a `<TemplateDir>` the
	/// loader pre-scans, or [`BsxTemplateRegistry::insert_source_from_path`]) before
	/// parsing the entry.
	///
	/// This is the single BSX entry-parse path: the unified
	/// [`TemplateLoader`](crate::prelude::TemplateLoader) dispatches `.bsx`/`.html`
	/// bytes here, and [`load_entry`](Self::load_entry) is a file convenience over it.
	pub fn parse_entry(world: &World, source: &str) -> Result<Self> {
		let mut roots = parse_document(source, &BsxParseConfig::bsx())?
			.into_iter()
			.filter(|node| match node {
				BsxNode::Comment(_) => false,
				BsxNode::Text(text) => !text.trim().is_empty(),
				_ => true,
			});
		let root = match (roots.next(), roots.next()) {
			(Some(root @ BsxNode::Element(_)), None) => root,
			(_, Some(_)) => bevybail!(
				"an entry document must have a single root element, found multiple roots"
			),
			_ => bevybail!("an entry document must have a single root element"),
		};
		let registry = world
			.get_resource::<BsxTemplateRegistry>()
			.cloned()
			.unwrap_or_default();
		Ok(Self::new(vec![root], registry))
	}

	/// Read and [`parse_entry`](Self::parse_entry) the entry document at `path`, a
	/// file convenience over the byte-oriented loader path.
	#[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
	pub fn load_entry(world: &World, path: impl AsRef<Path>) -> Result<Self> {
		let path = path.as_ref();
		let source = fs_ext::read_to_string(path)?;
		Self::parse_entry(world, &source).map_err(|err| {
			bevyhow!("failed to load entry `{}`: {err}", path.display())
		})
	}

	/// Build this template into a fresh root entity and flush, returning the root.
	///
	/// The flush materializes observer-driven structure (eg routes spawned off a
	/// component insert) before the caller inspects the tree.
	pub fn spawn(self, world: &mut World) -> Result<Entity> {
		let entity = world.spawn_template(self)?.id();
		world.flush();
		Ok(entity)
	}
}

#[cfg(all(test, feature = "fs", not(target_arch = "wasm32")))]
mod test {
	use crate::prelude::*;

	/// Write `source` to a temp `.bsx` file and return its path.
	fn entry_file(name: &str, source: &str) -> std::path::PathBuf {
		let path = fs_ext::workspace_root()
			.join("target/tests/bsx_entry")
			.join(name);
		fs_ext::write(&path, source).unwrap();
		path
	}

	#[crate::test]
	fn builds_root_element_into_entity() {
		let path = entry_file(
			"single.bsx",
			"<!-- entry -->\n<main class=\"app\"><span>hi</span></main>\n",
		);
		let mut world = TemplatePlugin::world();
		let root = BsxTemplate::load_entry(&world, &path)
			.unwrap()
			.spawn(&mut world)
			.unwrap();
		// the root element's component lands on the returned entity itself
		world
			.entity(root)
			.get::<Element>()
			.unwrap()
			.tag()
			.xpect_eq("main");
	}

	#[crate::test]
	fn rejects_multiple_roots() {
		let path = entry_file("multi.bsx", "<a/><b/>");
		let world = TemplatePlugin::world();
		// `BsxTemplate` is not `Debug`, so take the error without `unwrap_err`
		BsxTemplate::load_entry(&world, &path)
			.err()
			.unwrap()
			.to_string()
			.xpect_contains("single root element");
	}

	#[derive(Debug, Default, Component, Reflect)]
	#[reflect(Default, Component)]
	struct LinkedComp;

	/// A spread naming a component the running binary did not register (eg a lean
	/// http-only deploy loading a site whose `<Router {(.., TuiServer)}>` spread
	/// includes the dropped `tui` capability) is skipped, not fatal: the entry still
	/// builds and the registered components in the same spread still apply.
	#[crate::test]
	fn skips_unregistered_spread_component() {
		let mut world = TemplatePlugin::world();
		world.init_resource::<AppTypeRegistry>();
		world
			.resource::<AppTypeRegistry>()
			.write()
			.register::<LinkedComp>();

		let root = BsxTemplate::parse_entry(
			&world,
			"<div {(LinkedComp, NotLinkedHere)}/>",
		)
		.unwrap()
		.spawn(&mut world)
		.unwrap();

		// the registered component applied; the unregistered name was skipped.
		world.entity(root).get::<LinkedComp>().xpect_some();
	}

	#[derive(Debug, Default, Component, Reflect)]
	#[reflect(Default, Component)]
	struct SpreadComp {
		value: i64,
	}
	#[derive(Debug, Default, Resource, Reflect)]
	#[reflect(Default, Resource)]
	struct SpreadRes {
		value: i64,
	}

	/// Register `SpreadComp` + `SpreadRes` into a fresh template world.
	fn spread_world() -> World {
		let mut world = TemplatePlugin::world();
		world.init_resource::<AppTypeRegistry>();
		let registry = world.resource::<AppTypeRegistry>();
		let mut registry = registry.write();
		registry.register::<SpreadComp>();
		registry.register::<SpreadRes>();
		drop(registry);
		world
	}

	/// A `#[reflect(Resource)]` named in a spread declares the resource (patching a
	/// live one), never a component on the host; sibling components in the same
	/// spread still land on the host.
	#[crate::test]
	fn spread_resource_patches_live_and_keeps_host_clean() {
		let mut world = spread_world();
		world.insert_resource(SpreadRes { value: 1 });

		let root = BsxTemplate::parse_entry(
			&world,
			"<div {(SpreadRes{value:42}, SpreadComp{value:7})}/>",
		)
		.unwrap()
		.spawn(&mut world)
		.unwrap();

		// the resource field was patched in place.
		world.resource::<SpreadRes>().value.xpect_eq(42);
		// the sibling component landed on the host.
		world
			.entity(root)
			.get::<SpreadComp>()
			.unwrap()
			.value
			.xpect_eq(7);
		// the resource was NOT attached as a component on the host.
		world.entity(root).get::<SpreadRes>().xpect_none();
	}

	/// A spread resource on an absent resource spawns it from the type's default
	/// with the patched field applied over it.
	#[crate::test]
	fn spread_resource_spawns_absent_from_default() {
		let mut world = spread_world();

		BsxTemplate::parse_entry(&world, "<div {(SpreadRes{value:9})}/>")
			.unwrap()
			.spawn(&mut world)
			.unwrap();

		// absent before the build, now present with the patched field over default.
		world.resource::<SpreadRes>().value.xpect_eq(9);
	}
}
