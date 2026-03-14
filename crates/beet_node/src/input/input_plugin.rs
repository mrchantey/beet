use crate::prelude::*;
use beet_core::prelude::*;


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
			#[cfg(feature = "net")]
			commands.entity(element.entity).observe(navigate_navigator);
			#[cfg(not(feature = "net"))]
			commands
				.entity(element.entity)
				.observe(navigate_open_browser);
		}
		_ => {}
	}


	Ok(())
}


#[cfg(not(feature = "net"))]
fn navigate_open_browser(ev: On<PointerUp>, elements: ElementQuery) -> Result {
	fn is_external_url(href: &str) -> bool {
		href.contains("://")
			|| href.starts_with("mailto:")
			|| href.starts_with("tel:")
			|| href.starts_with("sms:")
			|| href.starts_with("javascript:")
			|| href.starts_with("data:")
	}
	let link = elements.get_as::<LinkView>(ev.event().target)?;
	if is_external_url(link.href) {
		if let Err(err) = webbrowser::open(link.href) {
			cross_log!("failed to open URL: {err}");
		}
	} else {
		todo!("internal navigation")
	}
	Ok(())
}

#[cfg(feature = "net")]
fn navigate_navigator(
	ev: On<PointerUp>,
	mut commands: Commands,
	elements: ElementQuery,
	navigators: Query<Entity, With<Navigator>>,
) -> Result {
	use beet_net::prelude::*;
	let link = elements.get_as::<LinkView>(ev.event().target)?;
	let url = Url::parse(link.href);
	commands
		.entity(navigators.single()?)
		.queue_async(|entity| Navigator::navigate_to(entity, url));

	Ok(())
}
