use beet::prelude::*;
use serde::Deserialize;
use serde::Serialize;

#[template]
#[derive(Serialize, Deserialize)]
pub fn ActionTest() -> impl Bundle {
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
pub fn RejectsNeg() -> impl Bundle {
	#[allow(unused_variables)]
	let (val, _set_val) = signal(0);
	#[allow(unused_variables)]
	let (get, set) = signal("Ready".to_string());

	let onclick = move |_: Trigger<OnClick>| {
		#[cfg(target_arch = "wasm32")]
		spawn_local(async move {
			let result =
				crate::actions::error_handling::reject_neg(val()).await;
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
