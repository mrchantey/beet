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

	match element.name() {
		"a" => {
			commands.entity(element.entity).observe(link_clicked);
		}
		_ => {}
	}


	Ok(())
}

fn link_clicked(ev: On<PointerUp>) {
	/// Returns `true` if the href is an external URL that should be
	/// opened by the OS rather than navigated to as a local path.
	fn is_external_url(href: &str) -> bool {
		href.contains("://")
			|| href.starts_with("mailto:")
			|| href.starts_with("tel:")
			|| href.starts_with("sms:")
			|| href.starts_with("javascript:")
			|| href.starts_with("data:")
	}

	fn on_click_link(
		ev: On<PointerUp>,
		mut commands: Commands,
		elements: ElementQuery,
		ancestors: Query<&ChildOf>,
	) {
		let link = elements.get_as::<LinkView>(ev.event().target)?;
		if is_external_url(link.href) {
			if let Err(err) = webbrowser::open(link.href) {
				cross_log!("failed to open URL: {err}");
			}
		} else {
			todo!("internal navigation")
		}
	}
}
