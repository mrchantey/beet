use beet::exports::bevy::ecs as bevy_ecs;
use beet::exports::bevy::reflect as bevy_reflect;
use beet::prelude::*;
use serde::Deserialize;


/// The metadata at the top of a markdown article,
#[derive(Debug, Default, Clone, Component, Deserialize)]
pub struct Article {
	pub title: String,
	pub created: Option<String>,
}



#[template]
#[derive(Reflect)]
pub fn ClientCounter() -> impl Bundle {
	rsx! {

		<div>hello</div>

	}
}

#[template]
pub fn ServerCounter(#[field(default = 0)] initial: i32) -> impl Bundle {
	#[allow(unused)]
	let (get, set) = signal(initial);

	let onclick = move |_: Trigger<OnClick>| {
		#[cfg(target_arch = "wasm32")]
		{
			spawn_local(async move {
				set(crate::prelude::actions::add(get(), 1).await.unwrap());
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
