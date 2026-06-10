//! The BSX event vocabulary: `bx:click=verb#field`.
//!
//! An event binds a mutation verb to a document field. `bx:click=increment#count`
//! installs a [`PointerDown`](crate::prelude::PointerDown) observer that runs the
//! verb against the bound field. The verb comes from a fixed vocabulary
//! ([`EventVerb`]); behavior beyond it is reached through the named-handler
//! escape hatch ([`BsxHandlerRegistry`]), a resource mapping a name to a Rust
//! installer, so the loader needs no compile-time knowledge of a concrete event
//! type.
//!
//! The field is bound by a [`FieldRef`] on the same entity as the observer, so a
//! verb mutates the entity's own [`Value`], which the document sync mirrors back
//! to the field, mirroring the `Increment` action without the `beet_net`
//! dependency.

use crate::prelude::*;
use beet_core::prelude::*;

/// A parsed `bx:click` binding: a verb run against a document field.
#[derive(Debug, Clone, PartialEq)]
pub struct EventBinding {
	/// The mutation verb, eg `increment`.
	pub verb: EventVerb,
	/// The field path the verb mutates, from the `#field` suffix.
	pub field: FieldPath,
	/// The field initializer from `#field=init`, if present.
	pub init: Option<Value>,
}

/// The fixed mutation vocabulary an event may bind to a document field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventVerb {
	/// `increment`: add one to a numeric field.
	Increment,
	/// `decrement`: subtract one from a numeric field.
	Decrement,
	/// `toggle`: flip a boolean field.
	Toggle,
	/// A named handler, resolved through [`BsxHandlerRegistry`] at install time.
	Named(SmolStr),
}

impl EventVerb {
	/// Parse a verb token, mapping unknown tokens to a named handler.
	pub fn parse(token: &str) -> Self {
		match token {
			"increment" => Self::Increment,
			"decrement" => Self::Decrement,
			"toggle" => Self::Toggle,
			other => Self::Named(other.into()),
		}
	}

	/// Run the verb against `value`, the observer's bound [`Value`].
	fn apply(&self, value: &mut Value) {
		match self {
			Self::Increment => {
				*value = Value::Int(value.as_i64().unwrap_or(0) + 1)
			}
			Self::Decrement => {
				*value = Value::Int(value.as_i64().unwrap_or(0) - 1)
			}
			Self::Toggle => {
				*value = Value::Bool(!matches!(value, Value::Bool(true)))
			}
			Self::Named(_) => {}
		}
	}
}

/// The escape hatch: a resource mapping a handler name to a Rust installer that
/// typically adds an observer to the event's entity.
///
/// A `bx:click=myhandler#field` whose verb is unknown resolves here at install
/// time. The installer receives the event entity and the bound field, so it can
/// wire arbitrary behavior the fixed verb set does not cover.
#[derive(Default, Resource)]
pub struct BsxHandlerRegistry {
	handlers: HashMap<SmolStr, BsxHandler>,
}

/// A named-handler installer: given the event entity and its field, wires
/// behavior (typically an observer) onto the entity.
pub type BsxHandler =
	std::sync::Arc<dyn Fn(&mut EntityWorldMut, &FieldPath) + Send + Sync>;

impl BsxHandlerRegistry {
	/// Register a named handler installer.
	pub fn insert(
		&mut self,
		name: impl Into<SmolStr>,
		handler: impl Fn(&mut EntityWorldMut, &FieldPath) + Send + Sync + 'static,
	) {
		self.handlers.insert(name.into(), std::sync::Arc::new(handler));
	}

	/// Look up a named handler installer.
	pub fn get(&self, name: &str) -> Option<BsxHandler> {
		self.handlers.get(name).cloned()
	}
}

/// Install an [`EventBinding`] onto `entity`: a [`FieldRef`] binding the field,
/// plus a [`PointerDown`] observer running the verb.
///
/// A fixed verb mutates the entity's own [`Value`] on click; an unknown verb
/// defers to a [`BsxHandlerRegistry`] installer if registered, else is a no-op
/// observer (so the loader never fails on an unknown handler).
pub fn install_event(entity: &mut EntityWorldMut, binding: &EventBinding) {
	let mut field = FieldRef::new(binding.field.clone());
	if let Some(init) = &binding.init {
		field = field.with_init(init.clone());
	}
	entity.insert(field);

	if let EventVerb::Named(name) = &binding.verb {
		let handler = entity
			.world_scope(|world| {
				world
					.get_resource::<BsxHandlerRegistry>()
					.and_then(|registry| registry.get(name))
			});
		if let Some(handler) = handler {
			handler(entity, &binding.field);
		}
		return;
	}

	let verb = binding.verb.clone();
	entity.observe(move |ev: On<PointerDown>, mut values: Query<&mut Value>| {
		if let Ok(mut value) = values.get_mut(ev.target) {
			verb.apply(&mut value);
		}
	});
}

#[cfg(test)]
mod test {
	use super::*;

	#[beet_core::test]
	fn parse_verbs() {
		EventVerb::parse("increment").xpect_eq(EventVerb::Increment);
		EventVerb::parse("toggle").xpect_eq(EventVerb::Toggle);
		EventVerb::parse("custom").xpect_eq(EventVerb::Named("custom".into()));
	}

	#[beet_core::test]
	fn increment_applies() {
		let mut value = Value::Int(4);
		EventVerb::Increment.apply(&mut value);
		value.xpect_eq(Value::Int(5));
	}

	#[beet_core::test]
	fn toggle_applies() {
		let mut value = Value::Bool(false);
		EventVerb::Toggle.apply(&mut value);
		value.xpect_eq(Value::Bool(true));
	}
}
