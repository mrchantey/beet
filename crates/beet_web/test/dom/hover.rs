use beet_web::prelude::*;
use sweet::*;

#[sweet_test]
pub fn works() -> Result<()> {
	append_html_for_tests();

	DomSim::<BeetWebNode>::default()
		.with_test_container(render_container())
		.with_node(Hover::new(1., 0.01))
		.run_forever()?;

	Ok(())
}
