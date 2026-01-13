use beet_core::prelude::*;
use serde_json::Value;

/// Common dump file path for debugging provider responses
pub const AI_DUMP_PATH: &str = "target/ai-dump.json";

/// Write JSON dump to the shared dump file for debugging
pub async fn write_dump(dump: &Vec<Value>) -> Result {
	fs_ext::write_async(
		AbsPathBuf::new_workspace_rel(AI_DUMP_PATH)?,
		serde_json::to_string_pretty(dump)?,
	)
	.await?;
	Ok(())
}

/// Update token usage on an entity
pub fn update_token_usage(
	entity: &mut bevy::ecs::world::EntityWorldMut,
	input_tokens: u64,
	output_tokens: u64,
) {
	let mut tokens = entity.get_mut::<crate::prelude::TokenUsage>().unwrap();
	tokens.input_tokens += input_tokens;
	tokens.output_tokens += output_tokens;
}
