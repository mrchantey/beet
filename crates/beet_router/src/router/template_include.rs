//! The `<Template src>` include: pull another entry in at the include site.
//!
//! Generalizes the old remote-template front-end. A no-code entry composes from
//! other files, so no single giant `main.bsx`: `src` names another entry of any
//! format (bsx, json, ron), read and parsed through the unified
//! [`EntryTemplate`], then built where the `<Template>` tag sits. Installed
//! through the [`BsxTagResolvers`] seam, so it overrides the core stub when a
//! router app is present.

use crate::prelude::*;
use beet_core::prelude::*;

/// Register the `<Template src="..">` include handler into the [`BsxTagResolvers`]
/// seam: a local `src` (resolved against the [`SiteRoot`], the entry's project
/// root) is read and its parsed entry built at the include site.
pub fn register_template_include(world: &mut World) {
	world.get_resource_or_init::<BsxTagResolvers>().insert(
		"Template",
		|el, entity| {
			let Some(src) = template_src(el) else {
				// `<Template>` with no `src` is a directives-only no-op.
				return Ok(());
			};
			// remote (`http(s)://`, `s3://`) includes resolve through the async
			// pending path so `LoadTemplate` waits for them; not yet wired to a real
			// `BlobStore` fetch (TODO).
			if is_remote(&src) {
				bevybail!(
					"remote `<Template src=\"{src}\">` includes are not yet \
					supported; use a local path"
				);
			}
			// resolve a local path against the SiteRoot (the including entry's dir),
			// else the cwd, then build the parsed entry into this site.
			let path = entity
				.world_scope(|world| {
					world.get_resource::<SiteRoot>().map(|root| root.0.clone())
				})
				.map(|dir| dir.join(src.as_str()))
				.map(Ok)
				.unwrap_or_else(|| AbsPathBuf::new(src.as_str()))?;
			let media = fs_ext::read_media(&path)?;
			let entry = entity.world_scope(|world| {
				EntryTemplate::from_bytes(world, &media)
			})?;
			entity.build_template(&entry)?;
			Ok(())
		},
	);
}

/// Whether `src` names a remote endpoint rather than a local path.
fn is_remote(src: &str) -> bool {
	src.starts_with("http://")
		|| src.starts_with("https://")
		|| src.starts_with("s3://")
}

/// The `src` string attribute of a `<Template>` element, if present.
fn template_src(el: &BsxElement) -> Option<SmolStr> {
	el.attributes.iter().find_map(|attr| {
		if attr.key != "src" {
			return None;
		}
		match &attr.value {
			AttrValue::Str(src) => Some(SmolStr::from(src.as_str())),
			_ => None,
		}
	})
}

#[cfg(all(test, feature = "json"))]
mod test {
	use super::*;

	/// Write `source` to a temp file and return its absolute path.
	fn temp_entry(name: &str, source: &str) -> AbsPathBuf {
		let path = fs_ext::workspace_root()
			.join("target/tests/template_include")
			.join(name);
		fs_ext::write(&path, source).unwrap();
		AbsPathBuf::new(path).unwrap()
	}

	/// An entry that includes a local `.bsx` and a local `.json` builds both at
	/// the include sites: each `<Template src>` becomes the included entry's root.
	#[beet_core::test]
	fn includes_local_bsx_and_json() {
		let mut world = TemplatePlugin::world();
		register_template_include(&mut world);
		world
			.resource::<AppTypeRegistry>()
			.write()
			.register::<Name>();

		// a bsx fragment and a json scene (a single `Name` node), included by path.
		let bsx = temp_entry("included.bsx", "<section class=\"card\"/>");
		let json_root = {
			let mut src = TemplatePlugin::world();
			src.resource::<AppTypeRegistry>().write().register::<Name>();
			let entity = src.spawn(Name::new("from-json")).id();
			TemplateSaver::new()
				.with_entity_tree(&src, entity)
				.save(&src, MediaType::Json)
				.unwrap()
		};
		let json = temp_entry("included.json", json_root.as_utf8().unwrap());

		let root = BsxTemplate::parse_entry(
			&world,
			&format!(
				"<main><Template src=\"{bsx}\"/><Template src=\"{json}\"/></main>"
			),
		)
		.unwrap()
		.spawn(&mut world)
		.unwrap();

		// the two includes built at their sites: a `section` element and a `Name`.
		let children: Vec<Entity> = world
			.entity(root)
			.get::<Children>()
			.unwrap()
			.iter()
			.collect();
		world
			.entity(children[0])
			.get::<Element>()
			.unwrap()
			.tag()
			.xpect_eq("section");
		world
			.entity(children[1])
			.get::<Name>()
			.unwrap()
			.as_str()
			.xpect_eq("from-json");
	}
}
