#![feature(more_qualified_paths)]

use beet_rsx::as_beet::*;
#[derive(Node)]
struct MyComponent {
	value: u32,
}
fn my_component(props: MyComponent) -> RsxRoot {
	rsx! { <div>{props.value}<slot /></div> }
}

fn main() {
	let str = rsx! {
		<div>
			<p>
				hello <MyComponent value=38>
					<div>some child</div>
				</MyComponent>
			</p>
		</div>
	}
	.pipe(RsxToHtmlString::default())
	.unwrap();

	assert_eq!(
		str,
		"<div><p>hello <div>38<div>some child</div></div></p></div>"
	);

	sweet::log!("success! {}", str);
}
