use beet::prelude::*;
use beet::rsx::sigfault::signal;
use serde::Deserialize;
use serde::Serialize;

#[derive(Node, Serialize, Deserialize)]
pub struct Calculator {
	#[field(default = 0)]
	initial: i32,
}

fn calculator(props: Calculator) -> RsxNode {
	let (get, set) = signal(props.initial);


	let get2 = get.clone();
	let onclick = move |_| {
		#[allow(unused)]
		let set = set.clone();
		#[allow(unused)]
		let get = get2.clone();
		#[cfg(target_arch = "wasm32")]
		{
			let val = get();
			beet::exports::wasm_bindgen_futures::spawn_local(async move {
				let result =
					crate::prelude::actions::add(val, 1).await.unwrap();
				set(result);
			});
		}
	};

	rsx! {
	<div>
		<Button
			variant=ButtonVariant::Outlined
			onclick=onclick>
			Server Cookie Count: {get.clone()}
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
