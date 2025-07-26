#![allow(unused)]
use crate::prelude::*;
use beet::exports::bevy::reflect as bevy_reflect;
use beet::prelude::*;

#[template]
#[derive(Reflect)]
pub fn ServerCounter(#[field(default = 0)] initial: i32) -> impl Bundle {
	let (get, set) = signal(initial);

	let onclick = move |_: Trigger<OnClick>| {
		#[cfg(target_arch = "wasm32")]
		{
			let val = get();
			beet::exports::wasm_bindgen_futures::spawn_local(async move {
				let result = actions::add(val, 1).await.unwrap();
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
