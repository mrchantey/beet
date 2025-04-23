// TODO
#![feature(more_qualified_paths)]
use beet_rsx::as_beet::*;

#[derive(Node)]
struct MyNode<T: 'static + Send + Sync + Clone + ToString> {
	value: T,
}
fn my_node<T: 'static + Send + Sync + Clone + ToString>(
	props: MyNode<T>,
) -> RsxNode {
	rsx! {{props.value}}
}

fn main() {
	// let str = rsx! {<MyNode<u32> value={3} />}
	// 	.xpipe(RsxToHtmlString::default())
	// 	.unwrap();
	// assert_eq!(str, "3");
	// sweet::log!("success!");
}
