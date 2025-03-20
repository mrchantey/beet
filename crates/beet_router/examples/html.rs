use anyhow::Result;
use beet_router::prelude::*;
use beet_rsx::prelude::*;


#[tokio::main]
async fn main() -> Result<()> {
	// usually its directly in src but test_site is a subdirectory
	// router.html_dir = PathBuf::from("crates/beet_router/target/client")
	// 	.canonicalize()
	// 	.unwrap();"
	// router.html_dir = "target/test_site".into();
	beet_router::test_site::routes::collect()
		.pipe(StaticRoutesToRsx::default())
		.await
		.unwrap()
		.pipe(ApplyRouteTemplates::new(
			"target/test_site/rsx-templates.ron",
		))?
		.pipe(RoutesToHtml::default())?
		.pipe(HtmlRoutesToDisk::new("target/test_site"))?;
	Ok(())
}
