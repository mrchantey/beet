use crate::prelude::*;
use bevy::prelude::*;



#[template]
pub fn ErrorText(value: DerivedGetter<Option<String>>) -> impl Bundle {
	rsx! {
		<div>
		{move ||
			if let Some(val) = value.get(){
				rsx!{<span>{val}</span>}.any_bundle()
			}else{
				().any_bundle()
			}
		}
		</div>
		<style>
			span{
				display: block;
				background-color: var(--bt-color-error);
				color: var(--bt-color-on-error);
				border-radius:var(--bt-border-radius);
				padding: 0 var(--bt-spacing);
			}
		</style>
	}
}
