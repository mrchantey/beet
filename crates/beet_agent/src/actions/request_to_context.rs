use crate::prelude::*;
use beet_core::prelude::*;
use beet_flow::prelude::*;
use beet_router::prelude::*;

#[derive(
	Debug,
	Clone,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	Hash,
	Deref,
	Reflect,
	Component,
)]
pub struct ModelRequestParams {
	#[deref]
	#[reflect(
		default,
		@ParamOptions::desc("Files to include in the context"))]
	file: Vec<String>,
}
impl Default for ModelRequestParams {
	fn default() -> Self { Self { file: default() } }
}


/// Loads request data into context.
///
/// This action reads the request from `RequestMeta` and spawns it as
/// user context for the AI model to process.
pub fn request_to_context() -> impl Bundle {
	OnSpawn::observe(
		|ev: On<GetOutcome>,
		 mut commands: Commands,
		 agent_query: AgentQuery<&RequestMeta>,
		 mut params: RouteParamQuery<ModelRequestParams>|
		 -> Result {
			let action = ev.target();
			let req_meta = agent_query.get(action)?;
			let query = req_meta.path().join(" ");

			let agent = agent_query.entity(action);

			context_spawner::spawn_user_text(
				&mut commands,
				agent,
				action,
				query,
			);
			for path in params.get(action)?.file.iter() {
				let ws_path = WsPathBuf::new(path);
				context_spawner::spawn_user_file(
					&mut commands,
					agent,
					action,
					ws_path,
				);
			}

			commands.entity(action).trigger_target(Outcome::Pass);

			Ok(())
		},
	)
}
