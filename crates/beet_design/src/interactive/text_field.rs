use crate::prelude::*;


#[derive(Default)]
pub enum TextFieldVariant {
	#[default]
	Outlined,
	Filled,
	Text,
}

impl TextFieldVariant {
	/// for use with `bt-c-textfield--<variant>`
	pub fn class_suffix(&self) -> &'static str {
		match self {
			TextFieldVariant::Outlined => "outlined",
			TextFieldVariant::Filled => "filled",
			TextFieldVariant::Text => "text",
		}
	}
}



/// A styled text field
#[template]
pub fn TextField(
	#[field(default)] variant: TextFieldVariant,
	#[field(flatten)] attrs: InputHtmlAttributes,
) -> impl Bundle {
	let mut attrs = attrs;
	attrs.push_class(format!(
		"bt-c-input bt-c-input--{}",
		variant.class_suffix()
	));

	rsx! {
		<div class="bt-c-input__container">
			<slot/>
			<input {attrs}>
		</div>
		<style src="./input.css" />
	}
}


/// A styled text area
#[template]
pub fn TextArea(
	#[field(default)] variant: TextFieldVariant,
	#[field(flatten)] attrs: TextAreaHtmlAttributes,
) -> impl Bundle {
	let mut attrs = attrs;
	attrs.push_class(format!(
		"bt-c-input bt-c-input--{}",
		variant.class_suffix()
	));

	rsx! {
		<div class="bt-c-input__container">
			<slot />
			<textarea {attrs}>
				<slot name="textarea" />
			</textarea>
		</div>
		<style src="./input.css" />
	}
}
