use beet_router::prelude::*;
use beet_rsx::prelude::*;


#[tokio::main]
async fn main() {
	let mut router = StaticFileRouter::default();
	// usually its directly in src but test_site is a subdirectory
	// router.html_dir = PathBuf::from("crates/beet_router/target/client")
	// 	.canonicalize()
	// 	.unwrap();"
	// router.html_dir = "target/test_site".into();
	beet_router::test_site::routes::collect_file_routes(&mut router);
	router
		.routes_to_rsx()
		.await
		.unwrap()
		.pipe(RoutesToHtml::new("target/test_site/rsx-templates.ron"))
		.unwrap()
		.pipe(HtmlRoutesToDisk::new("target/test_site"))
		.unwrap();
}
