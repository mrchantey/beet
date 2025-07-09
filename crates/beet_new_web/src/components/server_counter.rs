use beet::prelude::*;
use serde::Deserialize;
use serde::Serialize;

#[template]
#[derive(Serialize, Deserialize)]
pub fn ServerCounter(#[field(default = 0)] initial: i32) -> impl Bundle {
	#[allow(unused)]
	let (get, set) = signal(initial);

	let onclick = move |_: Trigger<OnClick>| {
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
		<button
			onclick=onclick>
			Server Counter: {get}
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
