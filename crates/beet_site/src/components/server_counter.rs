use crate::prelude::*;
use beet::prelude::*;

#[template]
#[derive(Reflect)]
pub fn ServerCounter(#[field(default = 0)] initial: i32) -> impl Bundle {
	let (get, set) = signal(initial);

	let onclick = move |_: On<OnClick>| {
		async_ext::spawn_local(async move {
			let value = actions::add(get(), 1)
				.await
				.expect("ServerCounter: failed to fetch from server");
			set(value);
		})
		.detach();
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
			flex-direction:column;
			gap: 1.em;
			padding:1.em 0.em;
		}
	</style>
	}
}
