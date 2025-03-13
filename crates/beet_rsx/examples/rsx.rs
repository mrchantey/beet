use beet_rsx::as_beet::*;

struct MyComponent {
	value: u32,
}
impl Component for MyComponent {
	fn render(self) -> RsxRoot {
		rsx! { <div>{self.value}<slot /></div> }
	}
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
