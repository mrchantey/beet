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
#[action]
#[derive(Default, Component, Reflect)]
pub struct PipelineSelector;

pub fn pipeline_selector(
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
