use beet_rsx::prelude::*;

fn main() {
	let template = rsx_template_ron! {
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
	sweet::log!("{}", template);
	let _ = ron::de::from_str::<RsxTemplateNode>(&template).unwrap();
	sweet::log!("success!");
}
