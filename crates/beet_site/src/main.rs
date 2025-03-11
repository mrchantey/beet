use anyhow::Result;
#[allow(unused)]
use beet::prelude::*;
#[allow(unused)]
use beet_site::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
	#[cfg(not(feature = "server"))]
	return build_static().await;

	#[cfg(feature = "server")]
	return BeetServer {
		public_dir: "target/client".into(),
		#[cfg(not(feature = "lambda"))]
		lambda: false,
		#[cfg(feature = "lambda")]
		lambda: true,
	}
	.serve()
	.await;
}


#[cfg(not(feature = "server"))]
async fn build_static() -> Result<()> {
	println!("rebuilding html files");
	let mut router = DefaultFileRouter::default();
	routes::collect_file_routes(&mut router);
	router.routes_to_html_files().await?;
	Ok(())
}
