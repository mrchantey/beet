use crate::prelude::*;
use beet_core::prelude::*;
use beet_flow::prelude::*;

/// Marks an action as a build pipeline with the given name.
#[derive(Debug, Clone, Component)]
pub struct BuildPipeline {
	/// The kebab-case name of the pipeline used as a cli argument,
	/// ie `all`, `client-only`, `pull-assets`
	pub name: String,
}

impl BuildPipeline {
	/// Creates a new [`BuildPipeline`] with the given name.
	pub fn new(name: &str) -> Self {
		Self {
			name: name.to_string(),
		}
	}
}

/// Runs the [`BuildPipeline`] with the name matching the
/// [`CliConfig::pipeline`], or the first if none specified.
/// If a pipeline is specified but not found an error is returned.
#[action(pipeline_selector)]
#[derive(Default, Component, Reflect)]
pub struct PipelineSelector;

fn pipeline_selector(
	mut ev: On<GetOutcome>,
	config: Res<CliConfig>,
	query: Query<&Children>,
	pipelines: Query<&BuildPipeline>,
) -> Result {
	let children = query.get(ev.action())?;
	if children.is_empty() {
		// no children, return error
		return Err(expect_action::to_have_children(&ev));
	}

	// if no pipeline specified, run the first child
	let Some(name) = &config.pipeline else {
		ev.trigger_action_with_cx(children[0], GetOutcome);
		return Ok(());
	};

	let pipelines = children
		.iter()
		.filter_map(|child| pipelines.get(child).map(|p| (child, p)).ok())
		.collect::<Vec<_>>();

	if let Some(pipeline) = pipelines
		.iter()
		.find(|(_, pipeline)| &pipeline.name == name)
	{
		// found the specified pipeline, run it
		ev.trigger_action_with_cx(pipeline.0, GetOutcome);
		Ok(())
	} else {
		// specified pipeline not found, return error
		let pipelines = pipelines
			.iter()
			.map(|(_, pipeline)| pipeline.name.as_str())
			.collect::<Vec<_>>()
			.join(", ");
		bevybail!(
			"
No build pipeline found with name '{name}'
available pipelines: {pipelines}"
		)
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_flow::prelude::*;
	use sweet::prelude::*;

	fn pipeline_tree() -> impl Bundle {
		(Name::new("root"), PipelineSelector::default(), children![
			(
				Name::new("child1"),
				BuildPipeline::new("first"),
				EndWith(Outcome::Pass)
			),
			(
				Name::new("child2"),
				BuildPipeline::new("second"),
				EndWith(Outcome::Pass)
			),
		])
	}

	#[test]
	fn runs_first_when_no_pipeline_specified() {
		let mut world = ControlFlowPlugin::world();

		// Insert CliConfig with no pipeline selected
		world.insert_resource(CliConfig {
			pipeline: None,
			launch_file: "launch.ron".into(),
			force_launch: false,
			package: None,
			launch_cargo_args: None,
			launch_no_default_args: false,
		});

		let on_run = collect_on_run(&mut world);
		let on_result = collect_on_result(&mut world);

		world
			.spawn(pipeline_tree())
			.trigger_target(GetOutcome)
			.flush();

		// Should run root then first child
		on_run
			.get()
			.xpect_eq(vec!["root".to_string(), "child1".to_string()]);

		on_result.get().xpect_eq(vec![
			("child1".to_string(), Outcome::Pass),
			("root".to_string(), Outcome::Pass),
		]);
	}

	#[test]
	fn runs_named_pipeline_when_specified() {
		let mut world = ControlFlowPlugin::world();
		// Insert CliConfig with pipeline "second"
		world.insert_resource(CliConfig {
			pipeline: Some("second".to_string()),
			launch_file: "launch.ron".into(),
			force_launch: false,
			package: None,
			launch_cargo_args: None,
			launch_no_default_args: false,
		});

		let on_run = collect_on_run(&mut world);
		let on_result = collect_on_result(&mut world);

		world
			.spawn(pipeline_tree())
			.trigger_target(GetOutcome)
			.flush();

		// Should run root then the named ("second") child
		on_run
			.get()
			.xpect_eq(vec!["root".to_string(), "child2".to_string()]);

		on_result.get().xpect_eq(vec![
			("child2".to_string(), Outcome::Pass),
			("root".to_string(), Outcome::Pass),
		]);
	}

	#[test]
	#[should_panic = "No build pipeline found with 'nonexistent'"]
	fn errors_when_pipeline_not_found() {
		let mut world = ControlFlowPlugin::world();

		// Insert CliConfig with pipeline name that doesn't exist
		world.insert_resource(CliConfig {
			pipeline: Some("nonexistent".to_string()),
			..default()
		});

		// Prepare observers before triggering
		let on_run = collect_on_run(&mut world);
		let on_result = collect_on_result(&mut world);

		// Spawn a PipelineSelector with two pipelines but none matching "nonexistent"
		world
			.spawn(pipeline_tree())
			.trigger_target(GetOutcome)
			.flush();

		// The pipeline name didn't match any child, so only the root should have run.
		on_run.get().xpect_eq(vec!["root".to_string()]);
		// No child produced a result.
		on_result.get().len().xpect_eq(0usize);
	}
}
