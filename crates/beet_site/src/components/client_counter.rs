use beet::prelude::*;

#[template]
#[derive(Reflect)]
pub fn ClientCounter(#[field(default = 0)] initial: i32) -> impl Bundle {
	let (get, set) = signal(initial);

	rsx! {
	<div>
		<Button
			variant=ButtonVariant::Outlined
			onclick=move |_| set(get() + 1)>
			Client Cookie Count: {get}
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
