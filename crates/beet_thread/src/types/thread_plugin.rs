use crate::o11s::ReasoningEffort;
use crate::o11s::ReasoningSummary;
use crate::o11s::request::ReasoningParam;
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_router::prelude::*;

#[derive(Default)]
pub struct ThreadPlugin {}

impl Plugin for ThreadPlugin {
	fn build(&self, app: &mut App) {
		app.init_plugin::<AsyncPlugin>()
			.init_plugin::<RouterPlugin>()
			.add_observer(insert_tool_definition)
			// ── Hierarchy types (needed for scene serialization) ──────────
			.register_type::<ChildOf>()
			.register_type::<Children>()
			// ── Uuid7 instantiations ─────────────────────────────────────
			.register_type::<Uuid7<Thread>>()
			.register_type::<Uuid7<Actor>>()
			.register_type::<Uuid7<Post>>()
			// ── Table types ───────────────────────────────────────────────
			.register_type::<Thread>()
			.register_type::<Actor>()
			.register_type::<ActorKind>()
			.register_type::<Post>()
			.register_type::<PostIntent>()
			.register_type::<Timestamp>()
			.register_type::<ResponseMeta>()
			// ── JSON reflect wrappers ─────────────────────────────────────
			.register_type::<JsonValue>()
			.register_type::<JsonMap>()
			// ── Streaming types ───────────────────────────────────────────
			.register_type::<EnvVar>()
			.register_type::<ModelDef>()
			.register_type::<O11sStreamer>()
			.register_type::<CompletionsStreamer>()
			// ── Reasoning sub-types ───────────────────────────────────────
			.register_type::<ReasoningEffort>()
			.register_type::<ReasoningSummary>()
			.register_type::<ReasoningParam>()
			// ── Tool definition types ─────────────────────────────────────
			.register_type::<ToolDefinition>()
			.register_type::<FunctionToolDefinition>()
			.register_type::<ProviderToolDefinition>()
			.register_type::<ToolChoice>()
			// ── Control-flow types (unit-input instantiations) ────────────
			.register_type::<ChildError>()
			.register_type::<CallOnSpawn<(), Outcome>>()
			.add_systems(Update, call_on_spawn::<(), Outcome>)
			.register_type::<Sequence<(), ()>>()
			.register_type::<Repeat<()>>()
			.register_type::<RepeatTimes<()>>()
			// ── SkipIfLatest wrapper instantiations ───────────────────────
			.register_type::<SkipIfLatest<StdinPost>>()
			.register_type::<SkipIfLatest<O11sStreamer>>();
	}
}
