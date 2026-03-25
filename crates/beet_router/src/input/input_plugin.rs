use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_node::prelude::*;


#[derive(Default)]
pub struct InputPlugin;


impl Plugin for InputPlugin {
	fn build(&self, app: &mut App) { app.add_observer(on_element_added); }
}


fn on_element_added(
	ev: On<Add, Element>,
	mut commands: Commands,
	query: ElementQuery,
) -> Result {
	let element = query.get(ev.entity)?;

	match element.tag() {
		"a" => {
			commands.entity(element.entity).observe(navigate_navigator);
		}
		_ => {}
	}


	Ok(())
}


fn navigate_navigator(
	ev: On<PointerUp>,
	mut commands: Commands,
	elements: ElementQuery,
	navigators: Query<Entity, With<Navigator>>,
) -> Result {
	let link = elements.get_as::<LinkView>(ev.event().target)?;
	let url = Url::parse(link.href);
	commands
		.entity(navigators.single()?)
		.queue_async(|entity| Navigator::navigate_to(entity, url));

	Ok(())
}
