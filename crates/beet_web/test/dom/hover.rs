use beet_web::prelude::*;
use sweet::*;

#[sweet_test]
pub fn works() -> Result<()> {
	append_html_for_tests();

	DomSim::<BeeNode>::default()
		.with_node(Hover::new(1., 0.01))
		.run_forever()?;

	Ok(())
}
