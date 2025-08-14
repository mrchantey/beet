use beet_core::prelude::*;
use bevy::prelude::*;


/// Load the client island scene from the html script if it exists,
/// for mounting and/or binding to the dom.
pub fn load_client_islands(world: &mut World) -> Result {
	let tag_name =
		&world.resource::<HtmlConstants>().client_islands_script_type;
	if let Some(scene) = beet_script_text(&tag_name)? {
		world.load_scene(scene)?;
	}

	Ok(())
}

fn beet_script_text(script_type: &str) -> Result<Option<String>> {
	use web_sys::window;

	let document = window().unwrap().document().unwrap();

	let Some(script) = document
		.query_selector(&format!(r#"script[type="{script_type}"]"#))
		.unwrap()
	else {
		return Ok(None);
	};

	let text = script
		.text_content()
		.ok_or_else(|| bevyhow!("Script tag has no text content"))?;

	Ok(Some(text))
}
