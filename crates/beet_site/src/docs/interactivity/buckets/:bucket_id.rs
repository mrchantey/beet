use beet::prelude::*;
use std::sync::Arc;


pub fn get(paths: Res<DynSegmentMap>) -> impl use<> + Bundle {
	rsx! { <Inner paths=paths.clone() client:load /> }
}


#[template]
#[derive(Reflect)]
pub fn Inner(paths: DynSegmentMap) -> impl Bundle {
	let _bucket = Bucket::new_local("buckets-demo");

	let bucket_id = paths
		.get("bucket_id")
		.cloned()
		.unwrap_or_else(|| "not found".to_string());

	rsx! {
		<div>
			howdy {bucket_id}
		</div>
	}
}
