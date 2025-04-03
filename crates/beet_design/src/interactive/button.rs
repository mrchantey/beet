use crate::prelude::*;


#[derive(Default)]
pub enum ButtonVariant {
	#[default]
	Primary,
	Secondary,
	Tertiary,
	Text,
	Outlined,
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
		}
	}
}


/// A styled button
#[derive(Node)]
pub struct Button {
	#[field(default)]
	pub variant: ButtonVariant,
	#[field(flatten)]
	pub attrs: ButtonHtmlAttributes,
}

fn button(Button { variant, mut attrs }: Button) -> RsxNode {
	let class = format!(" bt-c-button--{}", variant.class_suffix());

	attrs.push_class(csx!("bt-c-button", class));

	rsx! {
		<button {attrs}>
			<slot />
		</button>
		<style src="./button.css" />
	}
}




/// A button with no text, only an icon
#[derive(Node)]
pub struct IconButton {
	#[field(default=ButtonVariant::default())]
	pub variant: ButtonVariant,
}


fn icon_button(IconButton { variant }: IconButton) -> RsxNode {
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
#[derive(Node)]
pub struct Link {
	pub variant: ButtonVariant,
}


pub fn link(Link { variant }: Link) -> RsxNode {
	let class = format!("bt-c-button bt-c-button--{}", variant.class_suffix());
	rsx! {
		<a class=class>
			<slot />
		</a>
		<style src="./button.css" />
	}
}
