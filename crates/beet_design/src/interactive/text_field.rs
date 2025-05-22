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
#[derive(derive_template)]
pub struct TextField {
	#[field(default)]
	pub variant: TextFieldVariant,
	#[field(flatten=BaseHtmlAttributes)]
	pub attrs: InputHtmlAttributes,
}

fn text_field(TextField { variant, mut attrs }: TextField) -> WebNode {
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
#[derive(derive_template)]
pub struct TextArea {
	#[field(default)]
	pub variant: TextFieldVariant,
	#[field(flatten=BaseHtmlAttributes)]
	#[field(flatten=InputHtmlAttributes)]
	pub attrs: TextAreaHtmlAttributes,
}

fn text_area(TextArea { variant, mut attrs }: TextArea) -> WebNode {
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
