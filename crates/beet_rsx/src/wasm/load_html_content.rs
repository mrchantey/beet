use crate::prelude::*;
use beet_core::prelude::*;
use bevy::prelude::*;
use bevy::scene::ron;


/// Load the client island scene from the html script,
/// for mounting and/or binding to the dom.
pub fn load_client_islands(world: &mut World) -> Result {
	let tag_name =
		&world.resource::<HtmlConstants>().client_islands_script_type;
	let scene = beet_script_text(&tag_name)?;

	world.load_scene(scene)?;

	Ok(())
}

/// Update every stale [`TextNodeParent`] from the html document.
pub fn load_text_node_map(
	// registry: Res<AppTypeRegistry>,
	constants: Res<HtmlConstants>,
	mut query: Query<(&DomIdx, &mut TextNodeParent)>,
) -> Result {
	let map_str = beet_script_text(&constants.text_node_map_script_type)?;
	let map = ron::from_str::<TextNodeMap>(&map_str)?;
	// let registry = registry.read();
	// let reflect_deserializer = ReflectDeserializer::new(&registry);
	// let deserialized_value: Box<dyn PartialReflect> = reflect_deserializer
	// 	.deserialize(&mut ron::Deserializer::from_str(&map_str)?)?;
	// TextNodeMap::from_reflect(&*deserialized_value).ok_or_else(|| {
	// 	bevyhow!("Failed to deserialize text node map from script")
	// })?;


	for (dom_idx, parent) in map.0 {
		let mut ent_parent = query
			.iter_mut()
			.find(|(idx, _)| **idx == dom_idx)
			.map(|(_, parent)| parent)
			.ok_or_else(|| {
				bevyhow!("No entity found for dom index: {}", dom_idx)
			})?;
		beet_utils::log!("old: {:?}\nnew:{:?}", ent_parent, parent);
		*ent_parent = parent;
	}

	Ok(())
}

fn beet_script_text(script_type: &str) -> Result<String> {
	use web_sys::window;

	let document = window().unwrap().document().unwrap();

	let script = document
		.query_selector(&format!(r#"script[type="{script_type}"]"#))
		.unwrap()
		.ok_or_else(|| {
			bevyhow!("No script tag with type=\"bt-client-island-map\" found")
		})?;

	let text = script
		.text_content()
		.ok_or_else(|| bevyhow!("Script tag has no text content"))?;

	Ok(text)
}
