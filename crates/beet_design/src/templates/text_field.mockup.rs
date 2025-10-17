use crate::prelude::*;
use beet_core::prelude::*;

pub fn get() -> impl IntoHtml {
	rsx! { <Inner client:load /> }
}

// temp until global client:load
#[template]
#[derive(Reflect)]
pub fn Inner() -> impl Bundle {
	let (value, set_value) = signal("Hello world".to_string());

	#[cfg(target_arch = "wasm32")]
	effect(move || {
		beet::prelude::cross_log!("value: {}", value());
	});

	rsx! {
			<h2>Variants</h2>
			<div>
			<TextField
				oninput=move |e|set_value(e.value())
				variant=TextFieldVariant::Outlined
				value=value>	Outlined 	</TextField>
				<TextField
				oninput=move |e|set_value(e.value())
				variant=TextFieldVariant::Filled
				value=value>	Filled 		</TextField>
				<TextField
				oninput=move |e|set_value(e.value())
				variant=TextFieldVariant::Text
				value=value>	Text 			</TextField>
			</div>
			<h2>Disabled</h2>
			<div>
			<TextField
				disabled
				variant=TextFieldVariant::Outlined
				value=value>	Outlined 	</TextField>
			<TextField
				disabled
				variant=TextFieldVariant::Filled
				value=value>	Filled 		</TextField>
			<TextField
				disabled
				variant=TextFieldVariant::Text
				value=value>	Text 			</TextField>
			</div>
			<h2>Text Area</h2>
			<div>
			<TextArea
			oninput=move |e|set_value(e.value())
			variant=TextFieldVariant::Filled
			rows=10
			value=value>	Text Area 		</TextArea>
			</div>
			<style>
			div{
				padding:1.em;
				display: flex;
				flex-direction: row;
				align-items:flex-start;
				gap: 1rem;
			}
			</style>
	}
}
