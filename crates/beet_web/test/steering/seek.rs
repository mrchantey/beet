use beet::prelude::*;
use beet_web::prelude::*;
use sweet::*;

#[sweet_test]
pub async fn works() -> Result<()> {
	append_html_for_tests();
	DomSim::<BeeNode>::default()
		.with_node((Seek, FindSteerTarget::new("flower", 2.)))
		.run_forever()?;
	Ok(())
}
