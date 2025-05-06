use beet::design::exports::*;
use beet::prelude::*;

/// Provide the Beet Theme and Brand context to the application.
#[derive(Node)]
pub struct BeetContext {}

fn beet_context(_: BeetContext) -> WebNode {
	set_context(Brand {
		title: "Beet".into(),
		description: "A Rust web framework".into(),
		site_url: "https://beetrsx.dev".into(),
		version: env!("CARGO_PKG_VERSION").into(),
	});
	let theme = ThemeBuilder::with_source(Argb::new(255, 0, 255, 127)).build();

	rsx! {
		<DesignSystem theme=theme />
		<slot />
	}
}
