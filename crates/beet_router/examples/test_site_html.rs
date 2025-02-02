use beet_router::prelude::*;


#[tokio::main]
async fn main() {
	let mut router = DefaultFileRouter::default();
	beet_router::test_site::test_site_router::collect_file_routes(&mut router);
	router.routes_to_html_files().await.unwrap();
}
