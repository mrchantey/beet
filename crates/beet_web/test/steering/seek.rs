use beet::prelude::*;
use beet_web::prelude::*;
use sweet::*;

#[sweet_test]
pub async fn works() -> Result<()> {
	append_html_for_tests();
	AppOptions::default()
		.with_graph(BehaviorTree::new((Seek, FindSteerTarget::new("flower"))))
		.run();
	Ok(())
}
