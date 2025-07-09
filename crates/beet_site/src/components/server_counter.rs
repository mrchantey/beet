#[allow(unused)]
use crate::prelude::*;
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
			spawn_local(async move {
				set(actions::add(get(), 1).await.unwrap());
			});
		}
	};

	rsx! {
	<div>
		<Button
			variant=ButtonVariant::Outlined
			onclick=onclick>
			Server Cookie Count: {get}
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
