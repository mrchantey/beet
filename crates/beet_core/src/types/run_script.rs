//! [`RunScript`] and [`OnEvent`]: a script as data.
use crate::prelude::*;

/// A script: its program is the concatenation of its own [`Value`] and every
/// descendant's, depth first, self first, stopping at a descendant carrying
/// its own `RunScript` ([`ElementTextQuery::script_source`]). A javascript
/// document is one entity carrying it with its source as its `Value`, and in
/// markup `<RunScript>` is a raw-text element like `<script>`, its body the
/// source.
///
/// Data alone: `beet_action`'s scripting makes it a behaviour-tree leaf,
/// and with it the `world` bridge a script reaches the world through, so a
/// binary without scripting still loads and saves a scene holding one. A
/// script is behaviour and not content, so it is [`NotRendered`].
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Component, Reflect)]
#[reflect(Component, Default)]
#[require(NotRendered)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", reflect(Serialize, Deserialize))]
pub struct RunScript;

/// Names the event a script answers: the child a `bx:on<event>="source"`
/// directive lowers to, carrying [`RunScript`] and the source as its
/// [`Value`], so a saved document keeps its handlers. The plugin owning the
/// event runs it when the event reaches the child's parent; an event no
/// plugin owns leaves it inert.
#[derive(Debug, Default, Clone, PartialEq, Eq, Component, Reflect)]
#[reflect(Component, Default)]
#[require(RunScript)]
pub struct OnEvent {
	/// The event name, ie `click` for `bx:onclick`.
	pub event: SmolStr,
}

impl OnEvent {
	/// Answer the event named `event`.
	pub fn new(event: impl Into<SmolStr>) -> Self {
		Self {
			event: event.into(),
		}
	}
}
