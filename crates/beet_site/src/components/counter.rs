use beet::prelude::*;
use beet::rsx::sigfault::signal;
use serde::Deserialize;
use serde::Serialize;

#[derive(Node, Serialize, Deserialize)]
pub struct Counter {
	// #[field(default = 0)]
	initial: i32,
}

fn counter(props: Counter) -> RsxNode {
	let (get, set) = signal(props.initial);


	rsx! {
		<div>
			{get.clone()}
			<button onclick=move |_| { set(get() + 2) }>Increment</button>
		</div>
	}
}
