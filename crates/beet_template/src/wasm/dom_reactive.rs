use beet_common::node::TextNode;
use bevy::prelude::*;




pub fn dom_reactive_plugin(app: &mut App) {
	app.add_systems(Update, log_change);
}


fn log_change(query: Populated<&TextNode, Changed<TextNode>>) {
	for text_node in query.iter() {
		sweet::log!("TextNode changed: {:?}", text_node);
	}
}
