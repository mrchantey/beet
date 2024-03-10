use beet::prelude::*;
use beet_web::prelude::*;
use sweet::*;

#[sweet_test]
pub async fn works() -> Result<()> {
	append_html_for_tests();
	AppOptions::default()
		.with_graph(
			BehaviorTree::new(SequenceSelector)
				.with_child((
					Seek::default(),
					FindSteerTarget::new("flower"),
					// SetRunResult::success(),
					// SucceedOnArrive { radius: 0.1 },
				))
				.with_child(DespawnSteerTarget::default()),
		)
		.run();
	Ok(())
}
