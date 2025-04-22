use beet::prelude::*;
use beet::rsx::sigfault::signal;
use serde::Deserialize;
use serde::Serialize;

#[derive(Node, Serialize, Deserialize)]
pub struct Counter {
	#[field(default = 0)]
	initial: i32,
}

fn counter(props: Counter) -> RsxNode {
	let (get, set) = signal(props.initial);

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
