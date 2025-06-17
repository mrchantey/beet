use beet::prelude::*;
use beet::rsx::sigfault::signal;
use serde::Deserialize;
use serde::Serialize;

#[template]
#[derive(Serialize, Deserialize)]
pub fn Counter(#[field(default = 0)] initial: i32) -> impl Bundle {
	let (get, set) = signal(initial);

	let get2 = get.clone();
	rsx! {
	<div>
		<Button
			variant=ButtonVariant::Outlined
			onclick=move |_| set(get2() + 1)>
			Cookie Count: {get.clone()}
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
