//! The BSX event seam: `bx:<event>="script"`.
//!
//! An event binds a **script** to a trigger **event**.
//! `bx:click="target.with_field('count', count => count + 1)"` lowers to DATA
//! only: event `click`, plus the source it runs. Core knows neither the concrete event nor
//! how to evaluate a script, so neither picking nor an engine enters it.
//! Resolution is one registry lookup at build time:
//!
//! - [`EventRegistry`]: event name -> an installer that wires the trigger (eg a
//!   `PointerDown` observer) to run the script. The concrete installer lives
//!   where picking and the script backend are available (`beet_ui`/app) and is
//!   registered into this seam.
//!
//! A script is the whole vocabulary: it reaches the world through the same
//! capability-scoped `world`/`target` bridge every other beet script uses, so
//! adding a behavior is authoring a document, never recompiling the binary.
//! Mutating a document field is one such behavior (`target.with_field`), not a
//! structural special case.
//!
//! **`bx:` rather than `onclick`, deliberately.** `on*` is a real HTML attribute
//! and is passed through verbatim: it is not a directive, so it survives into
//! the rendered page for the browser to run in page scope, which is the
//! Astro-style sprinkling escape hatch. Overloading it would mean a handler the
//! browser executes where `world` and `target` do not exist, and would cost the
//! only way to emit a literal DOM handler. The `bx:` namespace means "beet
//! machinery, stripped before render", and that is exactly the distinction.

use crate::prelude::*;
use alloc::sync::Arc;

/// A parsed `bx:<event>="script"` binding: DATA only, resolved through the
/// [`EventRegistry`] at build time.
#[derive(Debug, Clone, PartialEq)]
pub struct EventBinding {
	/// The trigger event name, from the `bx:<event>` directive (eg `click`).
	pub event: SmolStr,
	/// The script source the trigger runs.
	pub script: SmolStr,
}

impl EventBinding {
	/// A binding running `script` under a trigger `event`.
	pub fn new(event: impl Into<SmolStr>, script: impl Into<SmolStr>) -> Self {
		Self {
			event: event.into(),
			script: script.into(),
		}
	}
}

/// An event installer: wires the trigger (typically an observer) onto `entity`,
/// running the script when the trigger fires.
///
/// The installer is where a concrete event type (eg a `PointerDown` observer),
/// picking, and the script backend live; core names none of them. It receives
/// the host entity and the script source.
pub(crate) type EventInstaller =
	Arc<dyn Fn(&mut EntityWorldMut, &str) + Send + Sync>;

/// The event seam: event name -> [`EventInstaller`]. Empty by default; an app
/// registers the concrete installers (eg `click`).
#[derive(Default, Resource)]
pub struct EventRegistry {
	installers: HashMap<SmolStr, EventInstaller>,
}

impl EventRegistry {
	/// Register an installer for an event name (eg `click`).
	pub fn insert(
		&mut self,
		name: impl Into<SmolStr>,
		installer: impl Fn(&mut EntityWorldMut, &str) + Send + Sync + 'static,
	) {
		self.installers.insert(name.into(), Arc::new(installer));
	}

	/// Look up an event installer by name.
	pub fn get(&self, name: &str) -> Option<EventInstaller> {
		self.installers.get(name).cloned()
	}
}

/// Install an [`EventBinding`] onto `entity`: wire the event's registered
/// trigger to run its script.
///
/// The trigger is resolved through the [`EventRegistry`]: a registered
/// installer wires it; an unregistered event is a graceful no-op, so a document
/// authored for a richer binary still loads in a leaner one, per the features
/// rule (behavior goes missing, structure does not).
pub(crate) fn install_event(
	entity: &mut EntityWorldMut,
	binding: &EventBinding,
) {
	let installer = entity.world_scope(|world| {
		world
			.get_resource::<EventRegistry>()
			.and_then(|registry| registry.get(&binding.event))
	});
	if let Some(installer) = installer {
		installer(entity, &binding.script);
	}
}

#[cfg(test)]
mod test {
	use super::*;

	/// A `click` installer that records the script it was handed, standing in
	/// for the real one (which needs picking and an engine).
	#[derive(Default, Resource, Deref)]
	struct RanScript(Vec<SmolStr>);

	fn register_recording_click(world: &mut World) {
		world.init_resource::<RanScript>();
		world.resource_mut::<EventRegistry>().insert(
			"click",
			|entity: &mut EntityWorldMut, script: &str| {
				let script = SmolStr::new(script);
				entity.world_scope(|world| {
					world.resource_mut::<RanScript>().0.push(script)
				});
			},
		);
	}

	#[crate::test]
	fn installs_the_authored_script() {
		let mut world = (BsxPlugin, DocumentPlugin).into_world();
		register_recording_click(&mut world);
		let binding = EventBinding::new("click", "target.set_field('c', 1)");
		let mut entity = world.spawn_empty();
		install_event(&mut entity, &binding);
		world
			.resource::<RanScript>()
			.first()
			.unwrap()
			.as_str()
			.xpect_eq("target.set_field('c', 1)");
	}

	/// An event nothing registered is a no-op: the entity still builds, it just
	/// carries no behavior.
	#[crate::test]
	fn an_unregistered_event_is_a_no_op() {
		let mut world = (BsxPlugin, DocumentPlugin).into_world();
		register_recording_click(&mut world);
		let binding = EventBinding::new("hover", "console.log('hi')");
		let mut entity = world.spawn_empty();
		install_event(&mut entity, &binding);
		world.resource::<RanScript>().is_empty().xpect_true();
	}
}
