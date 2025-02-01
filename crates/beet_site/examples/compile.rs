use beet::prelude::*;
use beet_site::prelude::*;

fn main() {
	// RoutesFileBuilder::default().build_and_write().unwrap();

	let routes = routes();

	for route in routes {
		let rsx = route.handler.into_rsx();
		let html = RsxToHtml::render_body(&rsx).to_string();
		println!("route: {}", html);
	}
}
