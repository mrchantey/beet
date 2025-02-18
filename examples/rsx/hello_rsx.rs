use beet::prelude::*;


fn main() {
	let value2 = "ld;rfs";
	let html = RsxToHtml::render_body(&rsx! {<div>lfdso {value2}</div> });
	println!("{}", html);
}
