use beet_core::prelude::*;
use bevy::prelude::*;


/// Load the client island scene from the html script,
/// for mounting and/or binding to the dom.
pub fn load_client_islands(world: &mut World) -> Result {
	let tag_name =
		&world.resource::<HtmlConstants>().client_islands_script_type;
	let scene = beet_script_text(&tag_name)?;

	world.load_scene(scene)?;

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
