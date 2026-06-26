//! Thread example wiring for the `beet` binary.
//!
//! The thread chat examples (`examples/thread/*.bsx`) run through the one binary
//! via `beet --main=examples/thread/chat.bsx`. This plugin adds the thread
//! runtime plus its charcell chat UI, and registers the example-specific inline
//! tool types so a scene's `<AgentChoiceAction/>` tag resolves from markup with
//! no per-example `.rs`.

use crate::beet::prelude::*;
use beet_core::prelude::*;

/// Adds the thread runtime ([`ThreadPlugin`]) and chat UI ([`ThreadUiPlugin`]),
/// and registers the thread examples' inline tool types, so a loaded thread
/// `.bsx` scene runs and its tool tags resolve.
pub struct ThreadExamplesPlugin;

impl Plugin for ThreadExamplesPlugin {
	fn build(&self, app: &mut App) {
		app.init_plugin::<ThreadPlugin>()
			.init_plugin::<ThreadUiPlugin>()
			// the tool_call example's inline tool, so `<AgentChoiceAction/>` resolves
			.register_type::<AgentChoiceAction>();
	}
}

/// Make a choice for what to do, following the schema. The `tool_call` example's
/// inline tool, referenced by tag in `examples/thread/tool_call.bsx`.
#[action(pure, route = "make-choice")]
#[derive(Component, Reflect)]
#[reflect(Component)]
fn AgentChoiceAction(cx: ActionContext<ChoiceInput>) -> String {
	match cx.choice {
		Choice::Attack => {
			"the attack was successful, you must feel very smug.."
		}
		Choice::Defend => "you exhibited cowardice, the shame..",
		Choice::GreetWarmly => {
			"its almost as if the glowing beet winked in response.."
		}
	}
	.to_string()
}

/// The schema the agent fills to call [`AgentChoiceAction`].
#[derive(Reflect, serde::Deserialize, serde::Serialize)]
struct ChoiceInput {
	/// The choice you can make, follow the schema and pick one.
	choice: Choice,
	/// A line of dialog to say as you make your choice
	catchphrase: String,
}

#[derive(Reflect, serde::Deserialize, serde::Serialize)]
enum Choice {
	Attack,
	/// Do Thing
	Defend,
	GreetWarmly,
}
