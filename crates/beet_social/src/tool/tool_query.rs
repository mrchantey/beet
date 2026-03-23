use beet_core::prelude::*;


/// Creates a tool name by combining the entity bits with the path.
/// Format: `tool_{entity_bits}_{path_with_underscores}`
pub fn tool_name(entity: Entity, path: &str) -> String {
	// unique as long as entity is not despawned
	// TODO to_bits better?
	let index = entity.index_u32();

	let sanitized_path = path
		.trim_start_matches('/')
		.replace('/', "_")
		.replace(':', "")
		.replace('*', "");
	if sanitized_path.is_empty() {
		format!("tool_{index}")
	} else {
		format!("tool_{index}_{sanitized_path}")
	}
}

/// Parses a tool name back into entity and path components.
/// Returns `None` if the name doesn't match the expected format.
pub fn parse_tool_name(world: &World, name: &str) -> Option<(Entity, String)> {
	let name = name.strip_prefix("tool_")?;
	let underscore_pos = name.find('_');

	let (index_str, path) = match underscore_pos {
		Some(pos) => {
			let (index, rest) = name.split_at(pos);
			(index, rest.trim_start_matches('_').replace('_', "/"))
		}
		None => (name, String::new()),
	};

	// TODO bits better?
	let index = index_str.parse().ok()?;

	let index = bevy_ecs::entity::EntityIndex::new(index);
	let entity = world.entities().resolve_from_index(index);
	Some((entity, format!("/{path}")))
}
