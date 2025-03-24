use beet_rsx::as_beet::*;

fn main() {
	let template = rsx_template! {
		<div key str="value" num=32 ident=some_val>
			<p>
				hello <MyComponent client:load some:src="32">
					<div>some child</div>
				</MyComponent>
			</p>
		</div>
	};
	sweet::log!("success! {:?}", template);
}
