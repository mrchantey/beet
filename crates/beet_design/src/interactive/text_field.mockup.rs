use crate::prelude::*;
use beet_rsx::sigfault::signal;


pub fn get() -> RsxNode {
	rsx! {<Inner client:load />}
}

// temp until global client:load
#[derive(Node, serde::Serialize, serde::Deserialize)]
pub struct Inner;

fn inner(_: Inner) -> RsxNode {
	let (value, set_value) = signal("Hello world".to_string());

	let set_value1 = set_value.clone();
	let set_value2 = set_value.clone();
	let set_value3 = set_value.clone();

	rsx! {
			<h2>Variants</h2>
			<div>
			<TextField
				onchange=move |e|set_value1(e.value())
				variant=TextFieldVariant::Outlined
				value=value.clone()>	Outlined 	</TextField>
				<TextField
				onchange=move |e|set_value2(e.value())
				variant=TextFieldVariant::Filled
				value=value.clone()>	Filled 		</TextField>
				<TextField
				onchange=move |e|set_value3(e.value())
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
