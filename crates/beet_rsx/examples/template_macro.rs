use beet_rsx::as_beet::beet;
use beet_rsx::prelude::*;

fn main() {
	let reverse_node = rsx_template! {
		<div
			key
			str="value"
			num=32
			ident=some_val
			>
			<p>hello
				<MyComponent>
					<div>some child</div>
				</MyComponent>
			</p>
		</div>
	};
	sweet::log!("success! {:?}", reverse_node);
}
