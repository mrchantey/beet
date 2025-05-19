use crate::prelude::*;
use beet_common::prelude::*;
use bevy::prelude::*;


pub fn extract_rsx_directives_plugin(app: &mut App) {
	app.add_systems(Update, extract_rsx_directives.after(rstml_to_node_tokens));
}



fn extract_rsx_directives(
	mut commands: Commands,
	query: Populated<(
		Entity,
		&AttributeOf,
		&AttributeKeyStr,
		Option<&AttributeValueStr>,
	)>,
) -> Result {
	for (entity, parent, key, value) in query.iter() {
		let key = key.as_str();
		let value = value.map(|v| v.as_str());
		if let Some(client_island) =
			ClientIslandDirective::try_from_attribute(key, value)
		{
			commands.entity(**parent).insert(client_island);
			commands.entity(entity).despawn();
		} else {
		}
	}
	Ok(())
}
