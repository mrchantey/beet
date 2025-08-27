use crate::prelude::*;
use beet::prelude::*;
use std::sync::Arc;




pub fn get() -> impl Bundle {
	rsx! { <Inner client:load /> }
}


#[template]
#[derive(Reflect)]
pub fn Inner() -> impl Bundle {
	let (items, set_items) = signal::<Vec<OnSpawnClone>>(default());
	let (on_change, trigger_change) = signal(());
	let (bucket, _) = signal(Bucket::new_local("buckets-demo"));

	#[cfg(feature = "client")]
	effect(move || {
		let _changed = on_change();

		async_ext::spawn_local(async move {
			let remove = Arc::new(move |path: RoutePath| {
				async_ext::spawn_local(async move {
					bucket().remove(&path).await.unwrap();
					trigger_change(());
				});
			});

			bucket()
				.list()
				.await
				.unwrap()
				.into_iter()
				.map(|path| {
					let item2 = path.clone();
					let remove = remove.clone();
					OnSpawnClone::insert(move || {
						let item = item2.clone();
						let remove = remove.clone();
						rsx! {
							<tr>
								<td>{item.to_string()}</td>
								<td>
									<Button
										variant=ButtonVariant::Outlined
										 onclick=move||{(remove.clone())(item2.clone())}>Remove</Button>
								</td>
							</tr>
						}
					})
				})
				.collect::<Vec<_>>()
				.xmap(set_items);
		});
	});

	rsx! {
		<h1>Buckets</h1>
		<p>This example uses local storage to manage a list of items</p>
		<Table>
		<tr slot="head">
			<td></td>
			<td></td>
			<td></td>
		</tr>
			<NewItem bucket=bucket/>
			{items}
		</Table>
	}
}
#[template]
fn NewItem(bucket: Getter<Bucket>) -> impl Bundle {
	let (name, set_name) = signal(String::new());

	let on_add = move || {
		// let timestamp = CrossInstant::unix_epoch().as_millis();
		// let path = RoutePath::new(format!("item-{timestamp}"));
		async_ext::spawn_local(async move {
			let path = name();
			bucket()
				.insert(&path.clone().into(), "hello world!")
				.await
				.unwrap();
			let route = routes::docs::interactivity::buckets::bucket_id(&path);
			navigate::to_page(&route);
		});
	};

	rsx! {
		<tr>
			<td>
				<TextField
					autofocus
					value={name}
					onchange=move |ev|{set_name(ev.value())}
						/>
			</td>
			<td>
				<Button onclick=move|| on_add()>Create</Button>
			</td>
		</tr>
	}
}
