// actions/add.rs
pub async fn get(input: JsonQuery<(i32, i32)>) -> Json<i32> {
	Json(input.0 + input.1)
}

// components/server_counter.rs
#[template]
pub fn ServerCounter(initial: i32) -> impl Bundle {
	let (get, set) = signal(initial);

	let onclick = move |_: Trigger<OnClick>| {
			spawn_local(async move {
				set(actions::add(get(), 1).await.unwrap());
			});
	};

	rsx! {
		<div>
			<Button
				variant=ButtonVariant::Outlined
				onclick=onclick>
				Server Cookie Count: {get}
			</Button>
		</div>
	}
}
