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
	let str =
		ron::ser::to_string_pretty(&reverse_node, Default::default()).unwrap();

	sweet::log!("{}", str);
}
