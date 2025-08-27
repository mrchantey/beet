use beet::prelude::*;
use std::sync::Arc;


pub fn get(paths: Res<DynSegmentMap>) -> impl use<> + Bundle {
	let bucket_id = paths.get("bucket_id").unwrap().clone();
	rsx! { <Inner bucket_id=bucket_id client:load /> }
}


#[template]
#[derive(Reflect)]
pub fn Inner(bucket_id: String) -> impl Bundle {
	let (bucket, _) = signal(Bucket::new_local("buckets-demo"));
	// let (content, set_content) = signal(None);

	// #[cfg(target_arch = "wasm32")]
	// effect(move || {
	// 	// let _changed = on_change();

	// 	async_ext::spawn_local(async move {
	// 		bucket()
	// 			.get(bucket_id)
	// 			.await
	// 			.unwrap()
	// 			.into_iter()
	// 			.map(async |path| {
	// 				let data = bucket().get(&path).await?;
	// 				Ok::<_, BevyError>((path, data))
	// 			})
	// 			.xmap(async_ext::try_join_all)
	// 			.await
	// 			.unwrap()
	// 			.into_iter()
	// 			.map(|(path, data)| {
	// 				let item2 = path.clone();
	// 				let remove = remove.clone();
	// 				OnSpawnClone::insert(move || {
	// 					let item = item2.clone();
	// 					let remove = remove.clone();
	// 					rsx! {
	// 						<tr>
	// 							<td>{item.to_string()}</td>
	// 							<td>{String::from_utf8_lossy(&data).to_string()}</td>
	// 							<td>
	// 								<Button onclick=move||{(remove.clone())(item2.clone())}>Remove</Button>
	// 							</td>
	// 						</tr>
	// 					}
	// 				})
	// 			})
	// 			.collect::<Vec<_>>()
	// 			.xmap(set_items);
	// 	});
	// });


	rsx! {
		<div>
			howdy {bucket_id}
		</div>
	}
}
