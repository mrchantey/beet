use crate::prelude::*;
use bevy::prelude::*;



#[template]
pub fn ErrorText() -> impl Bundle {
	rsx! {
		<div>
			<slot/>
		</div>
		<style>
			div{
				background-color: var(--bt-color-error);
				color: var(--bt-color-on-error);
			}
		</style>
	}
}
