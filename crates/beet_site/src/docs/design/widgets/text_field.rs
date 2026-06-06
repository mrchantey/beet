use crate::prelude::*;

/// Shows the [`TextField`] and [`TextArea`] widget variants.
///
/// The legacy live `oninput` signal demo and disabled variants are dropped; the
/// static variants render across web and terminal.
pub fn get() -> impl Scene {
	rsx! {
		<article>
			<h1>"Text Field"</h1>
			<h2>"Variants"</h2>
			<div {Classes::new(["design-row"])}>
				<TextField variant=TextFieldVariant::Outlined placeholder="Outlined"/>
				<TextField variant=TextFieldVariant::Filled placeholder="Filled"/>
				<TextField variant=TextFieldVariant::Text placeholder="Text"/>
			</div>
			<h2>"Text Area"</h2>
			<TextArea variant=TextFieldVariant::Filled placeholder="Text area"/>
		</article>
	}
}
