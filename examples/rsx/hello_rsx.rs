use beet::prelude::*;


fn main() {
	let planet = "world";
	let html = rsx! {<div>hello {planet}</div> }
		.xpipe(RsxToHtmlString::default())
		.unwrap();

	assert_eq!(html, "<div data-beet-rsx-idx=\"0\">hello world</div>");

	println!("{}", html);
}
