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
	// <button
	// 	onclick=move |_| set(get2() + 1)>
	// 	cookie count: {get.clone()}
	// </button>

	let get2 = get.clone();
	rsx! {
	<div>
	// <Nested/>
	its nested
	<Button
		variant=ButtonVariant::Secondary
		onclick=move |_| set(get2() + 1)>
		Numa of cookies: {get.clone()}
	</Button>
	</div>
	<style>
	div{
			display: flex;
			gap: 2rem;
		}
		</style>
	}
}


#[derive(Node, Serialize, Deserialize)]
struct Nested {
	// #[field(default = 0)]
	// initial: i32,
}
fn nested(_props: Nested) -> RsxNode {
	let (get, set) = signal(0);
	let get2 = get.clone();
	rsx! {
		<div>
			{4}
			<Button variant=ButtonVariant::Secondary onclick=move |_| set(get2() + 1)>
				Num cookies:
				{get.clone()}
			</Button>
		</div>
	}
}
