use beet::prelude::*;
use serde::Deserialize;
use serde::Serialize;

#[template]
#[derive(Serialize, Deserialize)]
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
