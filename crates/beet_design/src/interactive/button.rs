use crate::prelude::*;


#[derive(Default)]
pub enum ButtonVariant {
	#[default]
	Primary,
	Secondary,
	Tertiary,
	Text,
	Outlined,
	Error,
}

impl ButtonVariant {
	/// for use with `bt-c-button--<variant>`
	pub fn class_suffix(&self) -> &'static str {
		match self {
			ButtonVariant::Primary => "primary",
			ButtonVariant::Secondary => "secondary",
			ButtonVariant::Tertiary => "tertiary",
			ButtonVariant::Text => "text",
			ButtonVariant::Outlined => "outlined",
			ButtonVariant::Error => "error",
		}
	}
}


/// A styled button
#[template]
pub fn Button(
	#[field(default)] variant: ButtonVariant,
	#[field(flatten=BaseHtmlAttributes)] mut attrs: ButtonHtmlAttributes,
) -> impl Bundle {
	attrs.push_class(format!(
		"bt-c-button bt-c-button--{}",
		variant.class_suffix()
	));

	rsx! {
		<button {attrs}>
			<slot />
		</button>
		<style src="./button.css" />
	}
}




/// A button styled with no text, only an icon
#[template]
pub fn IconButton(
	#[field(default=ButtonVariant::default())] variant: ButtonVariant,
) -> impl Bundle {
	let class = format!(
		"bt-c-button bt-c-button--icon bt-c-button--{}",
		variant.class_suffix()
	);

	rsx! {
		<button class=class>
			<slot />
		</button>
		<style src="./button.css" />
	}
}

/// An anchor tag styled as a button
#[template]
pub fn Link(
	#[field(default)] variant: ButtonVariant,
	#[field(flatten=BaseHtmlAttributes)] mut attrs: AnchorHtmlAttributes,
) -> impl Bundle {
	let class = format!(" bt-c-button--{}", variant.class_suffix());
	attrs.push_class(csx!("bt-c-button", class));
	rsx! {
		<a {attrs}>
			<slot />
		</a>
		<style src="./button.css" />
	}
}
