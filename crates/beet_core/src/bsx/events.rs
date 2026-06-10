//! The BSX event/verb seam: `bx:<event>=<verb>#<field>`.
//!
//! An event binds a mutation **verb** to a document **field** via a trigger
//! **event**. `bx:click=increment#count` lowers to DATA only: event `click`,
//! verb `increment`, field `count`. Core knows neither the concrete event nor
//! the concrete verb, so picking never enters core. Resolution is a registry
//! lookup at build time, through two empty-by-default core registries:
//!
//! - [`EventRegistry`]: event name -> an installer that wires the trigger (eg a
//!   `PointerDown` observer). The concrete installer lives where picking is
//!   available (`beet_ui`/app) and is registered into this seam.
//! - [`VerbRegistry`]: verb name -> a verb handler. A verb mutates the bound
//!   document field and may need EXCLUSIVE world access, so the handler runs
//!   with an [`EntityWorldMut`] (an exclusive command, not inline in the
//!   observer).
//!
//! The field is bound by a [`FieldRef`] on the same entity, so a verb mutates
//! the entity's own [`Value`], which the document sync mirrors back to the
//! field. The example verb set (`increment`/`decrement`/`toggle`) and the
//! `click` installer are *registered* by an app, not built into core (see
//! `beet_ui`'s default registration).
//!
//! [`BsxHandlerRegistry`] remains the named-handler escape hatch for behavior
//! beyond a single field-mutating verb.

use crate::prelude::*;
use alloc::sync::Arc;

/// A parsed `bx:<event>=<verb>#field` binding: DATA only, resolved through the
/// [`EventRegistry`] and [`VerbRegistry`] at build time.
#[derive(Debug, Clone, PartialEq)]
pub struct EventBinding {
	/// The trigger event name, from the `bx:<event>` directive (eg `click`).
	pub event: SmolStr,
	/// The mutation verb name, from the value (eg `increment`).
	pub verb: SmolStr,
	/// The field path the verb mutates, from the `#field` suffix.
	pub field: FieldPath,
	/// The field initializer from `#field=init`, if present.
	pub init: Option<Value>,
}

/// A verb handler: mutates the bound entity's [`Value`] with exclusive world
/// access.
///
/// Exclusive access (an [`EntityWorldMut`]) is deliberate: a verb may need to
/// read or write beyond the entity's own [`Value`] (eg pull a target's input
/// value into the field) via [`EntityWorldMut::world_scope`], which an inline
/// observer closure cannot express. The event installer queues this to run as an
/// exclusive command, never inline.
pub type VerbFn = Arc<dyn Fn(&mut EntityWorldMut) + Send + Sync>;

/// An event installer: wires the trigger (typically an observer) onto `entity`,
/// running the named verb when the trigger fires.
///
/// The installer is where a concrete event type (eg a `PointerDown` observer)
/// and picking live; core never names one. It receives the entity, the verb
/// name to run (resolved against the [`VerbRegistry`] at fire time), and the
/// bound field.
pub type EventInstaller =
	Arc<dyn Fn(&mut EntityWorldMut, SmolStr, FieldPath) + Send + Sync>;

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
		installer: impl Fn(&mut EntityWorldMut, SmolStr, FieldPath)
		+ Send
		+ Sync
		+ 'static,
	) {
		self.installers.insert(name.into(), Arc::new(installer));
	}

	/// Look up an event installer by name.
	pub fn get(&self, name: &str) -> Option<EventInstaller> {
		self.installers.get(name).cloned()
	}
}

/// The verb seam: verb name -> [`VerbFn`]. Empty by default; an app registers
/// the example verb set (`increment`/`decrement`/`toggle`/…).
#[derive(Default, Resource)]
pub struct VerbRegistry {
	verbs: HashMap<SmolStr, VerbFn>,
}

impl VerbRegistry {
	/// Register a verb handler by name.
	pub fn insert(
		&mut self,
		name: impl Into<SmolStr>,
		verb: impl Fn(&mut EntityWorldMut) + Send + Sync + 'static,
	) {
		self.verbs.insert(name.into(), Arc::new(verb));
	}

	/// Look up a verb handler by name.
	pub fn get(&self, name: &str) -> Option<VerbFn> {
		self.verbs.get(name).cloned()
	}
}

/// The escape hatch: a resource mapping a handler name to a Rust installer that
/// typically adds an observer to the event's entity.
///
/// A `bx:click=myhandler#field` whose verb is registered neither as an event
/// installer nor a verb resolves here at install time. The installer receives
/// the event entity and the bound field, so it can wire arbitrary behavior the
/// fixed seam does not cover.
#[derive(Default, Resource)]
pub struct BsxHandlerRegistry {
	handlers: HashMap<SmolStr, BsxHandler>,
}

/// A named-handler installer: given the event entity and its field, wires
/// behavior (typically an observer) onto the entity.
pub type BsxHandler = Arc<dyn Fn(&mut EntityWorldMut, &FieldPath) + Send + Sync>;

impl BsxHandlerRegistry {
	/// Register a named handler installer.
	pub fn insert(
		&mut self,
		name: impl Into<SmolStr>,
		handler: impl Fn(&mut EntityWorldMut, &FieldPath) + Send + Sync + 'static,
	) {
		self.handlers.insert(name.into(), Arc::new(handler));
	}

	/// Look up a named handler installer.
	pub fn get(&self, name: &str) -> Option<BsxHandler> {
		self.handlers.get(name).cloned()
	}
}

/// Install an [`EventBinding`] onto `entity`: a [`FieldRef`] binding the field,
/// plus the event's registered trigger.
///
/// The field binding is always inserted (so the document sync mirrors the
/// entity's [`Value`]). The trigger is resolved through the [`EventRegistry`]:
/// a registered installer wires it (typically an observer running the named verb
/// from the [`VerbRegistry`]); an unregistered event falls back to the
/// [`BsxHandlerRegistry`] keyed by the verb name; an unresolved binding is a
/// graceful no-op (the loader never fails on an unknown event or verb).
pub fn install_event(entity: &mut EntityWorldMut, binding: &EventBinding) {
	let mut field = FieldRef::new(binding.field.clone());
	if let Some(init) = &binding.init {
		field = field.with_init(init.clone());
	}
	entity.insert(field);

	// a registered event installer wires the trigger + verb lookup.
	let installer = entity.world_scope(|world| {
		world
			.get_resource::<EventRegistry>()
			.and_then(|registry| registry.get(&binding.event))
	});
	if let Some(installer) = installer {
		installer(entity, binding.verb.clone(), binding.field.clone());
		return;
	}

	// fall back to the named-handler escape hatch keyed by the verb name.
	let handler = entity.world_scope(|world| {
		world
			.get_resource::<BsxHandlerRegistry>()
			.and_then(|registry| registry.get(&binding.verb))
	});
	if let Some(handler) = handler {
		handler(entity, &binding.field);
	}
}

#[cfg(test)]
mod test {
	use super::*;

	#[beet_core::test]
	fn binds_field_ref() {
		let mut world = (BsxPlugin, DocumentPlugin).into_world();
		let binding = EventBinding {
			event: "click".into(),
			verb: "increment".into(),
			field: FieldPath::new(["count"]),
			init: Some(Value::Int(0)),
		};
		let mut entity = world.spawn_empty();
		install_event(&mut entity, &binding);
		// the field binding is always installed, even with empty registries.
		entity.contains::<FieldRef>().xpect_true();
	}

	#[beet_core::test]
	fn registers_and_runs_verb() {
		let mut world = (BsxPlugin, DocumentPlugin).into_world();
		// register a `click` installer that runs the verb inline for the test.
		world.resource_mut::<EventRegistry>().insert(
			"click",
			|entity: &mut EntityWorldMut, verb: SmolStr, _field: FieldPath| {
				let verb_fn = entity.world_scope(|world| {
					world.resource::<VerbRegistry>().get(&verb)
				});
				if let Some(verb_fn) = verb_fn {
					verb_fn(entity);
				}
			},
		);
		world.resource_mut::<VerbRegistry>().insert(
			"increment",
			|entity: &mut EntityWorldMut| {
				if let Some(mut value) = entity.get_mut::<Value>() {
					*value = Value::Int(value.as_i64().unwrap_or(0) + 1);
				}
			},
		);

		let binding = EventBinding {
			event: "click".into(),
			verb: "increment".into(),
			field: FieldPath::new(["count"]),
			init: None,
		};
		let mut entity = world.spawn(Value::Int(4));
		install_event(&mut entity, &binding);
		entity.get::<Value>().unwrap().xpect_eq(Value::Int(5));
	}
}
