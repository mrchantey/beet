use beet::prelude::*;
use beet_web::prelude::*;
use sweet::*;

#[sweet_test]
pub async fn works() -> Result<()> {
	append_html_for_tests();
	DomSim::<BeetWebNode>::default()
		.with_test_container(render_container())
		.with_node((Seek, FindSteerTarget::new("flower", 2.)))
		.run_forever()?;
	Ok(())
}
