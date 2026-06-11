//! The native counterpart of the form demo's web `<script>` (which the terminal
//! skips): on [`Submit`] it pretty-prints the gathered values as JSON into the
//! `#form-output` element, mirroring the web
//! `JSON.stringify(FormData, null, 2)`.
//!
//! Everything generic (editable controls, gathering the fields, firing
//! [`Submit`]) lives upstream in [`FormPlugin`]; only this output binding is
//! site-specific.

use beet::prelude::*;

/// Registers the generic [`FormPlugin`] plus the demo's submit-to-JSON output.
pub struct FormRuntimePlugin;

impl Plugin for FormRuntimePlugin {
	fn build(&self, app: &mut App) {
		// init: the TUI target's `CharcellTuiPlugin` registers `FormPlugin` too
		app.init_plugin::<FormPlugin>()
			.add_observer(write_submit_json);
	}
}

/// On [`Submit`], pretty-print the carried values as JSON into `#form-output`,
/// the native analogue of the web `<script>`.
fn write_submit_json(
	ev: On<Submit>,
	elements: ElementQuery,
	mut commands: Commands,
) {
	let Ok(json) = ev.values.to_string_pretty() else {
		return;
	};
	if let Some((text, _)) = elements
		.iter()
		.find(|view| view.attribute_string("id") == "form-output")
		.and_then(|output| output.inner_text)
	{
		commands.entity(text).insert(Value::str(json));
	}
}
