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
	let (on_change, reload_items) = signal(());
	let (err, set_err) = signal::<Option<String>>(None);
	let (bucket, _) = signal(Bucket::new_local("buckets-demo"));

	#[cfg(feature = "client")]
	effect(move || {
		let _changed = on_change();

		async_ext::spawn_local(async move {
			bucket()
				.list()
				.await
				.unwrap()
				.into_iter()
				.map(|path| {
					OnSpawnClone::insert(move || {
						rsx! {
							<Item
								path=path
								bucket=bucket
								reload=reload_items
								set_err=set_err
							/>
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
		<ErrorText value={err}/>
		<Table>
		<tr slot="head">
			<td></td>
			<td></td>
		</tr>
			<NewItem
				bucket=bucket
				set_err=set_err
				reload=reload_items
				/>
			{items}
		</Table>
	}
}
#[template]
fn Item(
	path: RoutePath,
	bucket: Getter<Bucket>,
	set_err: Setter<Option<String>>,
	reload: Setter<()>,
) -> impl Bundle {
	let (path, _) = signal(path);
	let remove = move || {
		async_ext::spawn_local(async move {
			match bucket().remove(&path()).await {
				Ok(()) => {
					reload(());
				}
				Err(err) => set_err(Some(err.to_string())),
			}
		});
	};

	let route =
		routes::docs::interactivity::buckets::bucket_id(&path().to_string());

	rsx! {
		<tr>
			<td>{path()}</td>
			<td>
				<div>
				<Link
					variant=ButtonVariant::Outlined
					 href=route>Visit</Link>
				<Button
					variant=ButtonVariant::Error
					 onclick=move||remove()>Remove</Button>
						</div>
			</td>
		</tr>
		<style>
			div{
				display:flex;
				flex-direction:row;
				gap: var(--bt-spacing);
			}
		</style>
	}
}


#[template]
fn NewItem(
	bucket: Getter<Bucket>,
	reload: Setter<()>,
	set_err: Setter<Option<String>>,
) -> impl Bundle {
	let (name, set_name) = signal("my-object".to_string());

	let on_add = move || {
		async_ext::spawn_local(async move {
			let path = name();
			match bucket()
				.try_insert(&path.clone().into(), "hello world!")
				.await
			{
				Ok(()) => {
					reload(());
				}
				Err(err) => set_err(Some(err.to_string())),
			}
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
