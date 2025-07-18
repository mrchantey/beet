use crate::prelude::*;
use beet_core::prelude::*;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy::scene::ron;

/// When a [`TextNodeParent`] changes in the snippet, the wasm bundle needs
/// the updated component to accurately split joined nodes on signal changes
/// . We can do this by storing the updated component
/// in the html document.
#[derive(Reflect, serde::Serialize, serde::Deserialize)]
pub struct TextNodeMap(pub HashMap<DomIdx, TextNodeParent>);


pub fn apply_text_node_map(
	mut commands: Commands,
	// registry: Res<AppTypeRegistry>,
	constants: Res<HtmlConstants>,
	query: Query<Entity, Added<HtmlDocument>>,
	parents: Query<
		(&TextNodeParent, &DomIdx, &Children),
		Added<TextNodeParent>,
	>,
	children: Query<&Children>,
	signals: Query<&SignalReceiver<String>>,
) -> Result {
	for entity in query.iter() {
		let map = children
			.iter_descendants(entity)
			.filter_map(|child| {
				if let Ok((parent, dom_idx, children)) = parents.get(child) {
					if children.iter().any(|child| signals.contains(child)) {
						return Some((dom_idx.clone(), parent.clone()));
					}
				}
				None
			})
			.collect::<HashMap<_, _>>();

		// let registry = registry.read();
		// let reflect_serializer =
		// 	ReflectSerializer::new(map.as_partial_reflect(), &registry);
		let serde_map: String = ron::to_string(&TextNodeMap(map))?;

		commands.entity(entity).with_child((
			ElementNode::open(),
			NodeTag::new("script"),
			HtmlHoistDirective::Body,
			related!(
				Attributes[(
					AttributeKey::new("type"),
					AttributeLit::new(&constants.text_node_map_script_type),
				),]
			),
			children![TextNode::new(serde_map)],
		));
	}

	Ok(())
}
