use crate::prelude::*;
use beet_rsx::sigfault::signal;


pub fn get() -> RsxNode {
	rsx! {<Inner client:load />}
}

// temp until global client:load
#[derive(Node, serde::Serialize, serde::Deserialize)]
pub struct Inner;

fn inner(_: Inner) -> RsxNode {
	let (value,set_value) = signal("Hello world".to_string());
	rsx! {
			<h2>Variants</h2>
			<div>
			// <TextField 
			// 	variant=TextFieldVariant::Outlined value=value>	Outlined 	</TextField>
			// <TextField variant=TextFieldVariant::Filled 	value=value>	Filled 		</TextField>
			// <TextField variant=TextFieldVariant::Text 		value=value>	Text 			</TextField>
			// </div>
			// <h2>Disabled</h2>
			// <div>
			// <TextField disabled variant=TextFieldVariant::Outlined 	value=value>	Outlined 	</TextField>
			// <TextField disabled variant=TextFieldVariant::Filled 		value=value>	Filled 		</TextField>
			// <TextField disabled variant=TextFieldVariant::Text 			value=value>	Text 			</TextField>
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
