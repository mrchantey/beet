use beet::prelude::*;
use beet_web::prelude::*;
use sweet::*;

#[sweet_test]
pub async fn works() -> Result<()> {
	append_html_for_tests();
	DomSim::default()
		.with_graph((Seek, FindSteerTarget::new("flower", 2.)))
		.run()?;
	Ok(())
}
