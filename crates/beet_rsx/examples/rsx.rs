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
	let foo = rsx! {
		<div>
			<p>
				hello <MyComponent value=38>
					<div>some child</div>
				</MyComponent>
			</p>
		</div>
	};
	// let foo = rsx! {<div></div>};

	let str = RsxToHtml::render_body(&foo);
	assert_eq!(
		str,
		"<div><p>hello <div>38<div>some child</div></div></p></div>"
	);

	sweet::log!("success! {}", str);
}
