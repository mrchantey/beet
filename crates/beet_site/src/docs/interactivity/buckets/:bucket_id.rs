use beet::prelude::*;
use std::sync::Arc;


pub fn get() -> impl Bundle {
	rsx! { <Inner client:load /> }
}


#[template]
#[derive(Reflect)]
pub fn Inner(paths: Res<DynSegmentMap>) -> impl Bundle {
	let _bucket = Bucket::new_local("buckets-demo");

	rsx! {<div>howdy {paths.get("bucket_id").unwrap()}</div>

	}
}
