#![cfg_attr(not(feature = "client"), allow(unused))]
use crate::prelude::*;

pub fn get(paths: Res<DynSegmentMap>) -> impl use<> + Bundle {
	let bucket_id =
		paths.get("bucket_id").unwrap().clone().xmap(RoutePath::new);
	rsx! { <Inner bucket_id=bucket_id client:load /> }
}


#[template]
#[derive(Reflect)]
pub fn Inner(bucket_id: RoutePath) -> impl Bundle {
	let BucketItem {
		path,
		get_data,
		set_data,
		get_err,
		..
	} = Bucket::new_local("buckets-demo").item(bucket_id);

	let all_items = "/docs/design/templates/bucket_list";
	// let all_items = routes::docs::interactivity::buckets::index();


	rsx! {
		<div>
		<h1>{path.to_string()}</h1>
		<Link variant=ButtonVariant::Outlined href=all_items>All Items</Link>
			<ErrorText value={get_err}/>
		<TextArea
				autofocus
				value={move ||get_data().unwrap_or_default()}
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
