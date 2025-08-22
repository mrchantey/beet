use beet::prelude::*;





pub fn get() -> impl Bundle {
	rsx! { <Inner client:load /> }
}


#[template]
#[derive(Reflect)]
pub fn Inner() -> impl Bundle {
	let _provider = InMemoryProvider::new();

	// let bucket = Bucket::new(provider, "buckets-demo");





	rsx! {
		<div>loaded!</div>
	}
}
