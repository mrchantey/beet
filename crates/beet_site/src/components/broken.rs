use beet::prelude::*;
use serde::Deserialize;
use serde::Serialize;

#[derive(Node, Serialize, Deserialize)]
pub struct Broken;

fn broken(_props: Broken) -> WebNode {
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


#[derive(Node)]
struct RejectsNeg {}
fn rejects_neg(_props: RejectsNeg) -> WebNode {
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
