use beet::prelude::*;
use beet_site::prelude::*;


#[tokio::main]
async fn main() {
	let mut router = DefaultFileRouter::default();
	collect_file_routes(&mut router);
	let files = router.collect_rsx().await.unwrap();

	for (path, node) in files {
		let html = RsxToHtml::render_body(&node);
		println!("{}:\n{}", path, html);
	}
}
