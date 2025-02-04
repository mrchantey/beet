use beet_router::prelude::*;
use beet_rsx::rsx::RsxLocation;
use beet_rsx::rsx::RsxTemplateNode;
use std::collections::HashMap;


#[tokio::main]
async fn main() {
	let mut builder = BuildRsxTemplates::default();
	builder.src = "crates/beet_router/src/test_site".into();
	builder.dst = "target/test_site/rsx-templates.ron".into();
	// usually its directly in src but test_site is a subdirectory
	// router.dst_dir = PathBuf::from("crates/beet_router/target/client")
	// 	.canonicalize()
	// 	.unwrap();"

	// beet_router::test_site::routes::collect_file_routes(&mut router);
	// router.routes_to_html_files().await.unwrap();

	builder.build_and_write().unwrap();
	let tokens = builder.build_ron().unwrap();
	let map: HashMap<RsxLocation, RsxTemplateNode> =
		ron::de::from_str(&tokens.to_string()).unwrap();
	println!("wrote to {}\n{:#?}", builder.dst.display(), map);
}
