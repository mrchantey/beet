use crate::prelude::*;
use beet_core::prelude::*;




#[derive(Default, Component)]
pub struct CurrentCard;

/// Marker component added to an interface root to indicate that it is stateful like a UI,
/// as opposed to an interface only used for rendering and tool calls like a http server.
#[derive(Default, Component)]
pub struct StatefulInterface;

/// Runs on startup to visit the root node of an interface
pub(super) fn visit_root(
	query: Populated<&RouteTree, Added<StatefulInterface>>,
	mut commands: Commands,
) -> Result {
	let tree = query.single()?;

	let root = match tree.node() {
		Some(RouteNode::Card(CardNode { entity, .. })) => *entity,
		Some(RouteNode::Tool(tool)) => {
			bevybail!("Tui Server root node must be a card, found {:?}", tool)
		}
		None => {
			bevybail!("Tui Server must have a root node")
		}
	};
	commands.entity(root).insert(CurrentCard);
	Ok(())
}


/// Observer that ensures only one card at a time has the [`CurrentCard`] component.
pub(super) fn single_current_card(
	insert: On<Insert, CurrentCard>,
	query: Query<Entity, With<CurrentCard>>,
	mut commands: Commands,
) {
	for entity in query.iter() {
		if entity != insert.entity {
			commands.entity(entity).remove::<CurrentCard>();
		}
	}
}
