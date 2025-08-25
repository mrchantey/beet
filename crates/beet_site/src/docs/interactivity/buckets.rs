use beet::prelude::*;
use std::sync::Arc;





pub fn get() -> impl Bundle {
	rsx! { <Inner client:load /> }
}


#[template]
#[derive(Reflect)]
pub fn Inner() -> impl Bundle {
	let bucket = Bucket::new_local("buckets-demo");

	let (items, set_items) = signal::<Vec<OnSpawnClone>>(default());
	let (on_change, trigger_change) = signal(());
	let (bucket, _) = signal(bucket);

	#[cfg(target_arch = "wasm32")]
	effect(move || {
		let _changed = on_change();

		async_ext::spawn_local(async move {
			let remove = Arc::new(move |path: RoutePath| {
				// beet::log!("removing..");
				async_ext::spawn_local(async move {
					bucket().delete(&path).await.unwrap();
					trigger_change(());
				});
			});

			bucket()
				.list()
				.await
				.unwrap()
				.into_iter()
				.map(async |path| {
					let data = bucket().get(&path).await?;
					Ok::<_, BevyError>((path, data))
				})
				.xmap(async_ext::try_join_all)
				.await
				.unwrap()
				.into_iter()
				.map(|(path, data)| {
					let item2 = path.clone();
					let remove = remove.clone();
					OnSpawnClone::insert(move || {
						let item = item2.clone();
						let remove = remove.clone();
						rsx! {
							<tr>
								<td>{item.to_string()}</td>
								<td>{String::from_utf8_lossy(&data).to_string()}</td>
								<td>
									<Button onclick=move||{(remove.clone())(item2.clone())}>Remove</Button>
								</td>
							</tr>
						}
					})
				})
				.collect::<Vec<_>>()
				.xmap(set_items);
		});
	});

	let add_item = move |text: String| {
		let timestamp = CrossInstant::unix_epoch().as_millis();
		let path = RoutePath::new(format!("item-{timestamp}"));
		async_ext::spawn_local(async move {
			bucket().insert(&path, text).await.unwrap();
			trigger_change(());
		});
	};

	rsx! {
		<h1>Buckets</h1>
		<p>This example uses local storage to manage a list of items</p>
		<Table>
		<tr slot="head">
			<td></td>
			<td></td>
			<td></td>
		</tr>
			<NewItem add_item=add_item/>
			{items}
		</Table>
	}
}
#[template]
fn NewItem(
	add_item: Box<dyn 'static + Send + Sync + Fn(String)>,
) -> impl Bundle {
	let (description, set_description) = signal(String::new());

	let on_add = Arc::new(move || {
		add_item(description());
		set_description(Default::default());
	});

	rsx! {
		<tr>
			<td>
				<TextField
					autofocus
					value={description}
					onchange=move |ev|{set_description(ev.value())}
						/>
			</td>
			<td></td>
			<td>
				<Button onclick=move|| (on_add.clone())()>Create</Button>
			</td>
		</tr>
	}
}
