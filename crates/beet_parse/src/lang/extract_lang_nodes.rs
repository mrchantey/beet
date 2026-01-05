use beet_core::prelude::*;
use beet_dom::prelude::*;


/// For elements with a `script`, `style` or `code` tag, and without an
/// `node:inline` attribute, parse as a lang node:
/// - insert a [`ScriptElement`]
/// - insert a [`StyleElement`]
pub fn extract_lang_nodes(
	mut commands: Commands,
	query: Populated<(Entity, &NodeTag), Added<NodeTag>>,
	attributes: FindAttribute,
) {
	for (entity, tag) in query.iter() {
		// entirely skip node:inline
		if let Some((attr_ent, _)) = attributes.find(entity, "node:inline") {
			// its done its job, remove it
			commands.entity(attr_ent).despawn();
			continue;
		}
		// Insert the element type
		match tag.as_str() {
			"script" => {
				commands.entity(entity).insert(ScriptElement);
			}
			"style" => {
				commands.entity(entity).insert(StyleElement);
			}
			_ => {
				continue;
			}
		}
	}
}



#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_dom::prelude::*;

	#[test]
	fn works() {
		let mut world = World::new();
		let is_script = world.spawn(NodeTag::new("script")).id();
		let is_style = world.spawn(NodeTag::new("style")).id();
		let is_inline = world.spawn(NodeTag::new("style")).id();
		world.spawn((AttributeOf(is_inline), AttributeKey::new("node:inline")));

		world.run_system_cached(extract_lang_nodes).unwrap();

		world
			.entity(is_script)
			.contains::<ScriptElement>()
			.xpect_true();
		world
			.entity(is_style)
			.contains::<StyleElement>()
			.xpect_true();
		world
			.entity(is_inline)
			.contains::<StyleElement>()
			.xpect_false();
	}
}
