use beet::exports::bevy::reflect as bevy_reflect;
use beet::prelude::*;

#[template]
#[derive(Reflect)]
pub fn ClientCounter(#[field(default = 7)] initial: i32) -> impl Bundle {
	let (get, set) = signal(initial);

	rsx! {
	<div>
		<button
			onclick=move |_| set(get() + 1)>
			Client Counter: {get}
		</button>
	</div>
	<style>
		div {
			display: flex;
			align-items: center;
			gap: 1.em;
		}
	</style>
	}
}
