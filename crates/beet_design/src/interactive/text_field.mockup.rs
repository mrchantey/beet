use crate::prelude::*;


pub fn get() -> impl Bundle {
	rsx! { <Inner client:load /> }
}

// temp until global client:load
#[template]
#[derive(serde::Serialize, serde::Deserialize)]
pub fn Inner() -> impl Bundle {
	let (value, set_value) = signal("Hello world".to_string());

	let val2 = value.clone();
	effect(move || {
		beet::prelude::log!("value: {}", val2());
	});

	let set_value1 = set_value.clone();
	let set_value2 = set_value.clone();
	let set_value3 = set_value.clone();
	let set_value4 = set_value.clone();

	rsx! {
			<h2>Variants</h2>
			<div>
			<TextField
				oninput=move |e|set_value1(e.value())
				variant=TextFieldVariant::Outlined
				value=value.clone()>	Outlined 	</TextField>
				<TextField
				oninput=move |e|set_value2(e.value())
				variant=TextFieldVariant::Filled
				value=value.clone()>	Filled 		</TextField>
				<TextField
				oninput=move |e|set_value3(e.value())
				variant=TextFieldVariant::Text
				value=value.clone()>	Text 			</TextField>
			</div>
			<h2>Disabled</h2>
			<div>
			<TextField
				disabled
				variant=TextFieldVariant::Outlined
				value=value.clone()>	Outlined 	</TextField>
			<TextField
				disabled
				variant=TextFieldVariant::Filled
				value=value.clone()>	Filled 		</TextField>
			<TextField
				disabled
				variant=TextFieldVariant::Text
				value=value.clone()>	Text 			</TextField>
			</div>
			<h2>Text Area</h2>
			<div>
			<TextArea
			oninput=move |e|set_value4(e.value())
			variant=TextFieldVariant::Filled
			rows=10
			value=value.clone()>	Text Area 		</TextArea>
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
