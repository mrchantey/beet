//! The `.bsx` entrypoint: boot an app's entity tree from a single markup file.
//!
//! [`spawn_bsx_entry`](WorldBsxEntryExt::spawn_bsx_entry) parses a file like
//! `main.bsx` and builds its single root element *into* a fresh entity, so
//! root-level components (eg a `Router`) land on the returned entity exactly as
//! `world.spawn((Router, ..))` would place them. This differs from the
//! document-parse convention ([`BsxTemplate::container`]) where every root node
//! spawns as a child of a container.

use super::*;
use crate::prelude::*;
use std::path::Path;

/// World extension spawning a `.bsx` file as an app entrypoint.
#[extend::ext(name=WorldBsxEntryExt)]
pub impl World {
	/// Parse the file at `path` and build its single root element into a fresh
	/// entity, returning it.
	///
	/// Like an XML document element, exactly one root element is required;
	/// comments and whitespace at the root are ignored. `<path::to::X>` tags
	/// resolve against the current [`BsxTemplateRegistry`], so register any
	/// template directories (see
	/// [`register_bsx_templates`](WorldRegisterBsxExt::register_bsx_templates))
	/// before spawning the entry.
	fn spawn_bsx_entry(&mut self, path: impl AsRef<Path>) -> Result<Entity> {
		let path = path.as_ref();
		let source = fs_ext::read_to_string(path)?;
		let mut roots = parse_document(&source, &BsxParseConfig::bsx())?
			.into_iter()
			.filter(|node| match node {
				BsxNode::Comment(_) => false,
				BsxNode::Text(text) => !text.trim().is_empty(),
				_ => true,
			});
		let root = match (roots.next(), roots.next()) {
			(Some(BsxNode::Element(el)), None) => BsxNode::Element(el),
			(_, Some(_)) => bevybail!(
				"`{}` must have a single root element, found multiple roots",
				path.display()
			),
			_ => bevybail!(
				"`{}` must have a single root element",
				path.display()
			),
		};
		let registry = self
			.get_resource::<BsxTemplateRegistry>()
			.cloned()
			.unwrap_or_default();
		let entity = self
			.spawn_template(BsxTemplate::new(vec![root], registry))?
			.id();
		// flush so observer-driven structure (eg routes spawned off a component
		// insert) materializes before the caller inspects the tree.
		self.flush();
		Ok(entity)
	}
}

#[cfg(test)]
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
		let root = world.spawn_bsx_entry(&path).unwrap();
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
		let mut world = TemplatePlugin::world();
		world
			.spawn_bsx_entry(&path)
			.unwrap_err()
			.to_string()
			.xpect_contains("single root element");
	}
}
