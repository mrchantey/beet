use beet::prelude::*;
use beet_web::prelude::*;
use sweet::*;

#[sweet_test]
pub async fn works() -> Result<()> {
	append_html_for_tests();
	AppOptions::default()
		.with_graph(
			BehaviorTree::new((Repeat::default(), SequenceSelector))
				.with_child((
					Seek::default(),
					FindSteerTarget::new("flower", 0.5),
					SucceedOnArrive { radius: 0.1 },
				))
				.with_child((
					SetRunResult::success(),
					DespawnSteerTarget::default(),
				)),
		)
		.run();
	Ok(())
}
