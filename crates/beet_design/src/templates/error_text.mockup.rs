use crate::prelude::*;

pub fn get() -> impl Bundle {
	rsx! {
		<div>
		<ErrorText value=Some("this is an error".to_string())/>
		<div>below is an empty error</div>
			<ErrorText value=None></ErrorText>
		<div>that was an empty error</div>
		</div>
		<style>
			// div{
			// 	display:flex;
			// }
		</style>
	}
}
