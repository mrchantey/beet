use beet_router::prelude::*;


#[tokio::main]
async fn main() {
	let mut router = StaticFileRouter::default();
	// usually its directly in src but test_site is a subdirectory
	// router.html_dir = PathBuf::from("crates/beet_router/target/client")
	// 	.canonicalize()
	// 	.unwrap();"
	// router.html_dir = "target/test_site".into();
	beet_router::test_site::routes::collect_file_routes(&mut router);
	ExportHtml {
		html_dir: "target/test_site".into(),
		templates_map_path: "target/test_site/rsx-templates.ron".into(),
	}
	.routes_to_html_files(&mut router)
	.await
	.unwrap();
}
