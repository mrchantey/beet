use crate::prelude::*;

pub fn get() -> impl Bundle {
	rsx! {
		<div>
		<ErrorText>"this is an error"</ErrorText>
		<div>below is an empty error</div>
		<ErrorText></ErrorText>
		<div>that was an empty error</div>
		</div>
		<style>
			div{
				display:flex;
			}
		</style>
	}
}
