use crate::prelude::*;
use beet::prelude::*;
use beet::rsx::sigfault::signal;
use serde::Deserialize;
use serde::Serialize;
use sweet::prelude::*;

#[derive(Node, Serialize, Deserialize)]
pub struct ActionTest;

fn action_test(_props: ActionTest) -> RsxNode {
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
pub struct RejectsNeg {}
fn rejects_neg(_props: RejectsNeg) -> RsxNode {
	let (val, _set_val) = signal(0);
	let (get, set) = signal("Ready".to_string());

	let onclick = move |_| {
		let val = val.clone();
		let set = set.clone();
		spawn_local(async move {
			let result = actions::error_handling::reject_neg(val()).await;
			match result {
				Ok(_) => set("Success".into()),
				Err(e) => set(format!("Error: {}", e)),
			}
		});
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
			{get}
		</p>
	</div>
	<style>
		div {
			display: flex;
			flex-direction: row;
			align-items: center;
			gap: 1.em;
		}
	</style>
	}
}
