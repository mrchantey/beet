use beet::prelude::*;
use beet_web::prelude::*;
use sweet::*;

#[sweet_test]
pub async fn works() -> Result<()> {
	append_html_for_tests();
	DomSim::default().with_node(Wander).run_forever()?;

	Ok(())
}
