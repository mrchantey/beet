use beet::prelude::*;
use serde::Deserialize;
use serde::Serialize;

#[template]
pub fn Broken() -> impl Bundle {
	rsx! {
	<div>
		<RejectsNeg/>
	</div>
	<style>
		div {
			display: flex;
			flex-direction: column;
			gap: 1.em;
		}
	</style>
	}
}


#[template]
fn RejectsNeg() -> impl Bundle {
	let onclick = move |_| {
		sweet::log!("clicked");
	};

	rsx! {
		<h3>
		"rejects neg"
		</h3>
		<div>
		<Button
			variant=ButtonVariant::Outlined
			onclick=onclick>
			Trigger
		</Button>
		<p>
			{29}
		</p>
	</div>
	}
}
