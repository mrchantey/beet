use beet::prelude::*;
use beet::exports::Argb;
use beet::exports::ThemeBuilder;

#[template]
pub fn Layout() -> impl Bundle {
	let theme = ThemeBuilder::with_source(Argb::new(255, 0, 255, 127)).build();
	rsx! {
		<DesignSystem theme=theme />
		<ContentLayout>
			<slot/>
		</ContentLayout>
	}
}
