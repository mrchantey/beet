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
				display: block;
				background-color: var(--bt-color-error);
				color: var(--bt-color-on-error);
				border-radius:var(--bt-border-radius);
			}
		</style>
	}
}
// padding: 0 var(--bt-spacing);
