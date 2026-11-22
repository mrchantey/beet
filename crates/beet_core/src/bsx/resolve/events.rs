//! The BSX event directive: `bx:<event>="script"`.
//!
//! `bx:click="target.with_field('count', count => count + 1)"` lowers to data
//! only: a child of the element carrying [`OnEvent`] naming the event,
//! [`RunScript`] and the source as its [`Value`], so a saved document keeps
//! its handlers. Core knows neither the concrete event nor how to run a
//! script: the plugin that owns an event (`beet_ui`'s pointer input for
//! `click`) runs the child when the event reaches the element, and an event
//! no plugin owns leaves it inert, per the features rule (behaviour goes
//! missing, structure does not).
//!
//! A script is the whole vocabulary: it reaches the world through the same
//! capability-scoped `world`/`target` bridge every other beet script uses, so
//! adding a behaviour is authoring a document, never recompiling the binary.
//! Mutating a document field is one such behaviour (`target.with_field`), not
//! a structural special case.
//!
//! **`bx:` rather than `onclick`, deliberately.** `on*` is a real HTML
//! attribute and is passed through verbatim: it is not a directive, so it
//! survives into the rendered page for the browser to run in page scope, which
//! is the Astro-style sprinkling escape hatch. Overloading it would mean a
//! handler the browser executes where `world` and `target` do not exist, and
//! would cost the only way to emit a literal DOM handler. The `bx:` namespace
//! means "beet machinery, stripped before render", and that is exactly the
//! distinction.

use crate::prelude::*;

/// Lower a `bx:<event>="script"` directive on `entity`: spawn its
/// [`OnEvent`] child, the source as its [`Value`].
pub(crate) fn install_event(
	entity: &mut EntityWorldMut,
	event: &str,
	script: &str,
) {
	let host = entity.id();
	entity.world_scope(|world| {
		world.spawn((
			OnEvent::new(event),
			Value::Str(script.into()),
			ChildOf(host),
		));
	});
}

#[cfg(test)]
mod test {
	use super::*;

	#[crate::test]
	fn lowers_to_an_event_child() {
		let mut world = (BsxPlugin, DocumentPlugin).into_world();
		let mut entity = world.spawn_empty();
		install_event(&mut entity, "click", "target.set_field('c', 1)");
		let child = entity.get::<Children>().unwrap()[0];
		world
			.get::<OnEvent>(child)
			.unwrap()
			.event
			.as_str()
			.xpect_eq("click");
		world.get::<RunScript>(child).xpect_some();
		world
			.get::<Value>(child)
			.cloned()
			.unwrap()
			.xpect_eq(Value::Str("target.set_field('c', 1)".into()));
	}
}
