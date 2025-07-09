use beet::prelude::*;
use serde::Deserialize;
use serde::Serialize;

#[template]
#[derive(Serialize, Deserialize)]
pub fn ClientCounter(#[field(default = 0)] initial: i32) -> impl Bundle {
	let (get, set) = signal(initial);

	rsx! {
	<div>
		<Button
			variant=ButtonVariant::Outlined
			onclick=move |_| set(get() + 1)>
			Cookie Count: {get}
		</Button>
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
