use beet::prelude::*;
use beet_web::prelude::*;
use sweet::*;

#[sweet_test]
pub async fn works() -> Result<()> {
	append_html_for_tests();
	BeetWebApp::default()
		.with_test_container()
		.with_bundle(flower_bundle())
		.with_behavior(
			bee_bundle(),
			(Repeat::default(), SequenceSelector)
				.child((
					Seek::default(),
					FindSteerTarget::new("flower", 10.),
					SucceedOnArrive { radius: 0.1 },
				))
				.child((
					InsertOnRun(RunResult::Success),
					DespawnSteerTarget::default(),
				)),
		)
		.run_forever()?;
	Ok(())
}
