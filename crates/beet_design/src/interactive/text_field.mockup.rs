use crate::prelude::*;


pub fn get() -> RsxNode {
	let value = "Hello world";
	rsx! {
			<h2>Variants</h2>
			<div>
			<TextField variant=TextFieldVariant::Outlined value=value>	Outlined 	</TextField>
			<TextField variant=TextFieldVariant::Filled 	value=value>	Filled 		</TextField>
			<TextField variant=TextFieldVariant::Text 		value=value>	Text 			</TextField>
			</div>
			<h2>Disabled</h2>
			<div>
			<TextField disabled variant=TextFieldVariant::Outlined 	value=value>	Outlined 	</TextField>
			<TextField disabled variant=TextFieldVariant::Filled 		value=value>	Filled 		</TextField>
			<TextField disabled variant=TextFieldVariant::Text 			value=value>	Text 			</TextField>
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
