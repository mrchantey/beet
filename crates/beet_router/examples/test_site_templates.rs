use beet_router::prelude::*;


#[tokio::main]
async fn main() {
	let mut builder = BuildRsxTemplates::default();
	builder.src = "crates/beet_router/src/test_site".into();
	// usually its directly in src but test_site is a subdirectory
	// router.dst_dir = PathBuf::from("crates/beet_router/target/client")
	// 	.canonicalize()
	// 	.unwrap();"

	// beet_router::test_site::routes::collect_file_routes(&mut router);
	// router.routes_to_html_files().await.unwrap();

	builder.run().unwrap();
}
