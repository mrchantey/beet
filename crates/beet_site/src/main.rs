use beet::prelude::*;
use beet_site::prelude::*;









/// The main entry point for a beet server
#[tokio::main]
async fn main() {
	let mut router = DefaultFileRouter::default();
	collect_file_routes(&mut router);
	router.routes_to_html_files().await.unwrap();
}
