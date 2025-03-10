pub mod components;
pub mod routes;

pub mod prelude {
	pub use crate::components::*;

	pub use super::*;
}

use anyhow::Result;
use beet::prelude::*;

pub async fn build_static() -> Result<()> {
	println!("rebuilding html files");
	let mut router = DefaultFileRouter::default();
	routes::collect_file_routes(&mut router);
	router.routes_to_html_files().await?;
	Ok(())
}

// #[rustfmt::skip]
// pub fn beet_site_server() -> Router {
// 	Router::new()
// 		.merge(default_router_base())
// }
