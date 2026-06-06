use beet::prelude::*;

/// A complete [`Form`] showcase: a vertical stack of labelled fields styled
/// entirely by the rule set, plus a small script that echoes the submitted
/// values as JSON below the submit button.
///
/// The script is web-only (the terminal skips `<script>`); on the web it serves
/// as the live demo replacing the legacy `DynamicStruct` submit path.
pub fn get() -> impl Scene {
	rsx! {
		<article>
			<h1>"Form"</h1>
			<p>
				"A form with no custom styling: each field is a label over its input, "
				"stacked vertically with the submit button at the bottom."
			</p>
			<div {inline_class![(common_props::MaxWidth, Length::Rem(30.))]}>
			<Form name="demo">
				<label>"Name"</label>
				<TextField name="name" placeholder="Ada Lovelace"/>
				<label>"Email"</label>
				<TextField name="email" placeholder="ada@example.com"/>
				<label>"Role"</label>
				<Select name="role">
					<option value="engineer">"Engineer"</option>
					<option value="designer">"Designer"</option>
					<option value="teacher">"Teacher"</option>
				</Select>
				<label>"Message"</label>
				<TextArea name="message" placeholder="Tell us more…"/>
				<Button variant=ButtonVariant::Filled>"Submit"</Button>
			</Form>
			</div>
			<h2>"Submitted value"</h2>
			<pre><code id="form-output">"Submit the form to see its JSON value."</code></pre>
			<script>{FORM_SCRIPT}</script>
		</article>
	}
}

/// Serializes the demo form to pretty JSON and writes it into `#form-output`.
const FORM_SCRIPT: &str = r#"
document.querySelector('form[name="demo"]').addEventListener('submit', (event) => {
  event.preventDefault();
  const data = Object.fromEntries(new FormData(event.target).entries());
  document.getElementById('form-output').textContent = JSON.stringify(data, null, 2);
});
"#;
