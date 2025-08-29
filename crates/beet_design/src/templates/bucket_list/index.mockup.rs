use crate::prelude::*;
use beet::prelude::*;




pub fn get() -> impl Bundle {
	rsx! {
		<h1>Buckets</h1>
		<p>This example uses local storage to manage a list of items</p>
		<BucketList
			client:load
			route_prefix="/docs/design/templates/bucket_list"
			// route_prefix=routes::docs::design::templates::bucket_list::bucket_id("")
			bucket_name="buckets-demo"
		/>
	}
}
