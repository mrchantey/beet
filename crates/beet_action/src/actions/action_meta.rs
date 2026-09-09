use alloc::format;
use alloc::string::String;
use beet_core::prelude::*;
use bevy::reflect::TypeInfo;
use bevy::reflect::Typed;

/// Unified metadata for the one action an entity holds, combining
/// handler/input/output type information with optional reflection data,
/// description, and the extra signatures the entity's [`ActionOverload`]s match.
///
/// Constructed for [`Action::with_meta`], never inserted directly: [`Action`] is
/// its only producer, inserting it on add and removing it on remove, so a
/// hand-inserted one raises a clobber error when the real action lands.
///
/// Immutable, so every change is an insert: consumers observing
/// `Insert<ActionMeta>` (route discovery) see the registered overloads as well
/// as the canonical fields, whether an [`Action`] or an [`ActionOverload`]
/// landed last. An in-place edit would be invisible to them.
///
/// Created via [`ActionMeta::of`], optionally enriched by
/// [`with_type_info`](ActionMeta::with_type_info).
#[derive(Clone, Debug, Component, Get)]
#[component(immutable)]
pub struct ActionMeta {
	/// Type metadata for the action handler.
	handler: TypeMeta,
	/// Type metadata for the action input.
	input: TypeMeta,
	/// Type metadata for the action output.
	output: TypeMeta,
	/// Reflection data, present when the handler type implements [`Typed`].
	/// Input/output [`TypeInfo`] is optionally available when those types
	/// also implement [`Typed`].
	type_info: Option<ActionTypeInfo>,
	/// The additional `(input, output)` pairs this entity's
	/// [`ActionOverload`]s match.
	overloads: HashSet<(TypeMeta, TypeMeta)>,
}

/// Sentinel handler for an [`ActionMeta`] an [`ActionOverload`] created before
/// its entity's canonical [`Action`] landed. Private, so no real handler can
/// collide with it and [`Action`] can safely fill it in.
struct NoAction;

impl ActionMeta {
	/// Create an [`ActionMeta`] from explicit handler, input and output type parameters.
	pub fn of<H: 'static, In: 'static, Out: 'static>() -> Self {
		Self {
			handler: TypeMeta::of::<H>(),
			input: TypeMeta::of::<In>(),
			output: TypeMeta::of::<Out>(),
			type_info: None,
			overloads: default(),
		}
	}

	/// Attach whatever reflection data the three types turned out to support.
	///
	/// The `#[action]` macro's seam: it cannot test a trait bound, so it probes
	/// each of `Self`, `In` and `Out` with [`MaybeTyped`] at the call site and
	/// hands the answers here. A handler that reflects yields a doc description;
	/// an input or output that reflects yields a schema too, independently.
	pub fn with_type_info(
		mut self,
		handler: Option<&'static TypeInfo>,
		input: Option<&'static TypeInfo>,
		output: Option<&'static TypeInfo>,
	) -> Self {
		// the description hangs off the handler, so no handler info is no info
		self.type_info = handler.map(|handler_info| ActionTypeInfo {
			handler_info,
			input_info: input,
			output_info: output,
		});
		self
	}

	/// An [`ActionMeta`] with no canonical action yet, created by an
	/// [`ActionOverload`] whose entity's [`Action`] has not landed. Filled in by
	/// [`Action`]'s insert hook, so overload registration is order-independent.
	pub(crate) fn unset() -> Self { Self::of::<NoAction, NoAction, NoAction>() }

	/// Whether this meta is still waiting for its canonical action, see
	/// [`unset`](Self::unset).
	pub(crate) fn is_unset(&self) -> bool {
		self.handler == TypeMeta::of::<NoAction>()
	}

	/// Adopt overloads already registered on the meta this one replaces, so
	/// [`Action`] can refresh the canonical fields (and re-fire `Insert`) without
	/// dropping its [`ActionOverload`] registrations.
	pub(crate) fn with_overloads(
		mut self,
		overloads: HashSet<(TypeMeta, TypeMeta)>,
	) -> Self {
		self.overloads.extend(overloads);
		self
	}

	/// Register an additional signature this action matches, see
	/// [`ActionOverload`].
	pub(crate) fn insert_overload<In: 'static, Out: 'static>(&mut self) {
		self.overloads
			.insert((TypeMeta::of::<In>(), TypeMeta::of::<Out>()));
	}

	/// Deregister a signature previously added by
	/// [`insert_overload`](Self::insert_overload).
	pub(crate) fn remove_overload<In: 'static, Out: 'static>(&mut self) {
		self.overloads
			.remove(&(TypeMeta::of::<In>(), TypeMeta::of::<Out>()));
	}

	/// Whether this action matches an `(In, Out)` call, either as its canonical
	/// signature or through a registered [`ActionOverload`].
	///
	/// The single meta-matching predicate: call resolution, sequence child
	/// validation and the child selector all ask this.
	pub fn matches<In: 'static, Out: 'static>(&self) -> bool {
		let pair = (TypeMeta::of::<In>(), TypeMeta::of::<Out>());
		(self.input, self.output) == pair || self.overloads.contains(&pair)
	}

	/// The canonical signature plus any overloads, for diagnostics.
	pub fn signatures(&self) -> String {
		self.overloads.iter().fold(
			format!("{} -> {}", self.input, self.output),
			|acc, (input, output)| format!("{acc}, {input} -> {output}"),
		)
	}

	/// The full type name of the handler function or type.
	pub fn name(&self) -> &'static str { self.handler.type_name() }

	/// Returns true if the output type matches `T`.
	pub fn output_is<T: 'static>(&self) -> bool {
		self.output.type_id() == core::any::TypeId::of::<T>()
	}

	/// The handler [`TypeInfo`], if reflection data is available.
	pub fn handler_info(&self) -> Option<&'static TypeInfo> {
		self.type_info.map(|info| info.handler_info)
	}

	/// The input [`TypeInfo`], if full reflection data is available.
	pub fn input_info(&self) -> Option<&'static TypeInfo> {
		self.type_info.and_then(|info| info.input_info)
	}

	/// The output [`TypeInfo`], if full reflection data is available.
	pub fn output_info(&self) -> Option<&'static TypeInfo> {
		self.type_info.and_then(|info| info.output_info)
	}

	/// A description from doc comments, if reflection data is available.
	pub fn description(&self) -> Option<&str> {
		self.type_info.as_ref().and_then(|info| info.description())
	}

	/// JSON schema for the input type, if full reflection data is available.
	///
	/// # Errors
	/// Returns an error if the input type's schema names something JSON Schema
	/// cannot express, see [`JsonSchema::try_from_schema`].
	#[cfg(feature = "json")]
	pub fn input_json_schema(&self) -> Result<Option<JsonSchema>> {
		self.type_info
			.and_then(|info| info.input_info)
			.map(JsonSchema::from_type_info)
			.transpose()
	}

	/// JSON schema for the output type, if full reflection data is available.
	///
	/// # Errors
	/// Returns an error if the output type's schema names something JSON Schema
	/// cannot express, see [`JsonSchema::try_from_schema`].
	#[cfg(feature = "json")]
	pub fn output_json_schema(&self) -> Result<Option<JsonSchema>> {
		self.type_info
			.and_then(|info| info.output_info)
			.map(JsonSchema::from_type_info)
			.transpose()
	}

	/// Assert that the provided types match this action's input/output types.
	///
	/// # Errors
	/// Returns an error if types don't match.
	pub fn assert_match<In: 'static, Out: 'static>(&self) -> Result {
		let expected_input = self.input();
		let expected_output = self.output();
		let received_input = TypeMeta::of::<In>();
		let received_output = TypeMeta::of::<Out>();
		if *expected_input != received_input {
			bevybail!(
				"Action Call Input mismatch.\nExpected: {}\nReceived: {}.",
				expected_input,
				received_input,
			);
		} else if *expected_output != received_output {
			bevybail!(
				"Action Call Output mismatch.\nExpected: {}\nReceived: {}.",
				expected_output,
				received_output,
			);
		} else {
			Ok(())
		}
	}
}

/// Reflection metadata for an action. Always includes the handler
/// [`TypeInfo`]; input and output [`TypeInfo`] are optional and
/// present only when created vian [`ActionTypeInfo::of_full`].
#[derive(Debug, Copy, Clone)]
pub struct ActionTypeInfo {
	/// The handler [`TypeInfo`].
	handler_info: &'static TypeInfo,
	/// The input [`TypeInfo`], if available.
	input_info: Option<&'static TypeInfo>,
	/// The output [`TypeInfo`], if available.
	output_info: Option<&'static TypeInfo>,
}

impl ActionTypeInfo {
	/// The handler [`TypeInfo`].
	pub fn handler_info(&self) -> &'static TypeInfo { self.handler_info }
	/// The input [`TypeInfo`], if available.
	pub fn input_info(&self) -> Option<&'static TypeInfo> { self.input_info }
	/// The output [`TypeInfo`], if available.
	pub fn output_info(&self) -> Option<&'static TypeInfo> { self.output_info }

	/// A description from the handler's doc comments, if available.
	pub fn description(&self) -> Option<&str> {
		cfg_if! {
			if #[cfg(feature = "reflect")] {
				self.handler_info.docs()
			} else {
				None
			}
		}
	}
}

/// Lightweight type metadata using [`TypeId`](core::any::TypeId) for
/// comparison and [`type_name`](core::any::type_name) for display.
#[derive(Debug, Copy, Clone)]
pub struct TypeMeta {
	type_name: &'static str,
	type_id: core::any::TypeId,
}

impl TypeMeta {
	/// Create a [`TypeMeta`] for the given type.
	pub fn of<T: 'static>() -> Self {
		Self {
			type_name: core::any::type_name::<T>(),
			type_id: core::any::TypeId::of::<T>(),
		}
	}
	pub fn of_val<T: 'static>(_: &T) -> Self { Self::of::<T>() }

	/// The full type name, ie `core::option::Option<i32>`.
	pub fn type_name(&self) -> &'static str { self.type_name }
	/// The [`TypeId`](core::any::TypeId) for this type.
	pub fn type_id(&self) -> core::any::TypeId { self.type_id }
}

impl core::fmt::Display for TypeMeta {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		write!(f, "{}", self.type_name)
	}
}

/// Identity is the [`TypeId`](core::any::TypeId) alone; the name is display data
/// derived from it.
impl PartialEq for TypeMeta {
	fn eq(&self, other: &Self) -> bool { self.type_id == other.type_id }
}
impl Eq for TypeMeta {}

impl core::hash::Hash for TypeMeta {
	fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
		self.type_id.hash(state);
	}
}

/// Probes whether `T` reflects, so a macro can pick the richest metadata it
/// can without the author declaring which one applies.
///
/// Rust has no way to branch on a trait bound, so this branches on *method
/// resolution* instead: the `Typed` impl sits behind one autoref and wins when
/// its bound holds, otherwise resolution falls through to the blanket impl on
/// the value itself. Call it through the double reference:
///
/// ```
/// # use beet_action::prelude::*;
/// # use beet_core::prelude::*;
/// let some = (&&MaybeTyped::<u32>::new()).maybe_type_info();
/// let none = (&&MaybeTyped::<fn()>::new()).maybe_type_info();
/// some.xpect_some();
/// none.xpect_none();
/// ```
///
/// The answer is decided where the call is written, so a generic action whose
/// param carries no `Typed` bound resolves to `None` for every instantiation —
/// the honest answer, since nothing at that site knows better.
pub struct MaybeTyped<T>(core::marker::PhantomData<fn() -> T>);

impl<T> Default for MaybeTyped<T> {
	fn default() -> Self { Self::new() }
}

impl<T> MaybeTyped<T> {
	/// A probe for `T`.
	pub fn new() -> Self { Self(core::marker::PhantomData) }
}

/// The reflecting arm of [`MaybeTyped`], reached through one autoref.
pub trait MaybeTypedReflect {
	/// `T`'s [`TypeInfo`].
	fn maybe_type_info(&self) -> Option<&'static TypeInfo>;
}
impl<T: Typed> MaybeTypedReflect for &MaybeTyped<T> {
	fn maybe_type_info(&self) -> Option<&'static TypeInfo> {
		Some(T::type_info())
	}
}

/// The fallback arm of [`MaybeTyped`], reached when the `Typed` bound fails.
pub trait MaybeTypedFallback {
	/// `None`, since `T` does not reflect here.
	fn maybe_type_info(&self) -> Option<&'static TypeInfo>;
}
impl<T> MaybeTypedFallback for MaybeTyped<T> {
	fn maybe_type_info(&self) -> Option<&'static TypeInfo> { None }
}
