use crate::prelude::*;


#[derive(Default)]
pub enum SelectVariant {
	#[default]
	Outlined,
	Filled,
	Text,
}

impl SelectVariant {
	pub fn class_suffix(&self) -> &'static str {
		match self {
			SelectVariant::Outlined => "outlined",
			SelectVariant::Filled => "filled",
			SelectVariant::Text => "text",
		}
	}
}




/// A styled select element
#[template]
pub fn Select(
	#[field(default)] variant: SelectVariant,
	#[field(flatten=BaseHtmlAttributes)] mut attrs: InputHtmlAttributes,
) -> impl Bundle {
	attrs.push_class(format!(
		"bt-c-select bt-c-select--{}",
		variant.class_suffix()
	));

	rsx! {
		<select {attrs}>
			<Button></Button>
			<slot />
		</select>
		<style src="./select.css" />
	}
}
