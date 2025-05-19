#![feature(more_qualified_paths)]
use beet_rsx::as_beet::*;

#[derive(Node)]
struct MyNode;

fn my_node(_props: MyNode) -> WebNode { Default::default() }



fn main() {
	let str = rsx! {
		<div key1 key2="val" key3=3>
			hello
			<MyNode />
		</div>
	}
	.xpipe(RsxToHtmlString::default())
	.unwrap();
	assert_eq!(
		str,
		"<div><p data-beet-rsx-idx=\"3\">is_optional: None</p><p data-beet-rsx-idx=\"8\">is_required: 38</p><p data-beet-rsx-idx=\"13\">is_default: 7</p><p data-beet-rsx-idx=\"18\">is_generic_default: Foo(PhantomData<u32>)</p><p data-beet-rsx-idx=\"23\">is_into: \"foobar\"</p></div>"
	);
	sweet::log!("success!");
}
