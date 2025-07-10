use beet::exports::Argb;
use beet::exports::ThemeBuilder;
use beet::prelude::*;

/// Provide the Beet Theme and Brand context to the application.
#[template]
pub fn BeetContext() -> impl Bundle {
	ReactiveApp::insert_resource(Brand {
		title: "Beet".into(),
		description: "A Rust web framework".into(),
		site_url: "https://beetstack.dev".into(),
		version: env!("CARGO_PKG_VERSION").into(),
	});
	let theme = ThemeBuilder::with_source(Argb::new(255, 0, 255, 127)).build();

	rsx! {
		<DesignSystem theme=theme />
		<slot />
	}
}
