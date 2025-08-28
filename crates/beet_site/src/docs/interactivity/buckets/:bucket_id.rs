use beet::prelude::*;
use std::sync::Arc;

use crate::prelude::routes;


pub fn get(paths: Res<DynSegmentMap>) -> impl use<> + Bundle {
	let bucket_id =
		paths.get("bucket_id").unwrap().clone().xmap(RoutePath::new);
	rsx! { <Inner bucket_id=bucket_id client:load /> }
}


#[template]
#[derive(Reflect)]
pub fn Inner(bucket_id: RoutePath) -> impl Bundle {
	let (bucket, _) = signal(Bucket::new_local("buckets-demo"));
	let (bucket_id, _) = signal(bucket_id);
	let (data, set_data) = signal::<Option<String>>(None);
	let (err, set_err) = signal::<Option<String>>(None);

	// initialize the data
	#[cfg(feature = "client")]
	effect(move || {
		async_ext::spawn_local(async move {
			match bucket().get(&bucket_id()).await {
				Ok(data) => {
					let data = String::from_utf8_lossy(&data).to_string();
					beet::log!("got data: {data}");
					set_data(Some(data))
				}
				Err(err) => set_err(Some(err.to_string())),
			}
		});
	});


	// update local storage with the new data
	#[cfg(feature = "client")]
	effect(move || {
		if let Some(data) = data() {
			async_ext::spawn_local(async move {
				bucket().insert(&bucket_id(), data).await.unwrap();
			});
		}
	});

	let all_items = routes::docs::interactivity::buckets::index();

	rsx! {
		<div>
		<h1>{bucket_id().to_string()}</h1>
		<Link variant=ButtonVariant::Outlined href=all_items>All Items</Link>
			<ErrorText value={err}/>
		<TextArea
				autofocus
				value={move ||data().unwrap_or_default()}
				oninput=move |ev|{set_data(Some(ev.value()))}
				rows=40
			/>
		</div>
		<style>
			div{
				display:flex;
				flex-direction:column;
				gap:var(--bt-spacing);
			}
		</style>
	}
}
