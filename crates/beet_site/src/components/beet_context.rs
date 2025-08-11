use beet::exports::Argb;
use beet::exports::ThemeBuilder;
use beet::prelude::*;

/// Provide the Beet Theme and Brand context to the application.

#[template]
pub fn BeetContext() -> impl Bundle {
	let theme = ThemeBuilder::with_source(Argb::new(255, 0, 255, 127)).build();

	rsx! {
		<DesignSystem theme=theme />
		<slot />
	}
}
