use crate::prelude::*;
use beet::prelude::*;

#[template]
#[cfg_attr(feature = "server", allow(unused))]
#[derive(Reflect)]
pub fn BucketList(
	#[field(into)] bucket_name: String,
	route_prefix: String,
) -> impl Bundle {
	let (items, set_items) = signal::<Vec<OnSpawnClone>>(default());
	let (on_change, reload_items) = signal(());
	let route_prefix = getter(route_prefix.trim_end_matches("/").to_string());
	let (err, set_err) = signal::<Option<String>>(None);
	let bucket = getter(Bucket::new_local(bucket_name));

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
								route_prefix=route_prefix()
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
	route_prefix: String,
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

	let route = format!("{route_prefix}{}", path());
	// routes::docs::interactivity::buckets::bucket_id(&.to_string());

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
