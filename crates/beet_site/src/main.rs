use beet::prelude::*;

/// The main entry point for a beet server
#[tokio::main]
async fn main() {
	println!("rebuilding html files");
	let mut router = DefaultFileRouter::default();
	beet_site::routes::collect_file_routes(&mut router);
	router.routes_to_html_files().await.unwrap();
}
