//! The BSX event/verb seam: `bx:<event>=verb{ arg: value, .. }`.
//!
//! An event binds a named **verb** to a trigger **event** with schema-verified
//! named **arguments**. `bx:click=increment{ field: @doc:count, amount: 3 }`
//! lowers to DATA only: event `click`, verb `increment`, args `field` (an `@doc`
//! binding) and `amount` (a literal). Core knows neither the concrete event nor
//! the concrete verb, so picking never enters core. Resolution is a registry
//! lookup at build time, through two empty-by-default core registries:
//!
//! - [`EventRegistry`]: event name -> an installer that wires the trigger (eg a
//!   `PointerDown` observer). The concrete installer lives where picking is
//!   available (`beet_ui`/app) and is registered into this seam.
//! - [`VerbRegistry`]: verb name -> a [`VerbSchema`] (its named args) plus a
//!   [`VerbFn`]. A verb runs with full world access via an [`EntityWorldMut`] on
//!   the event host, so it may read or write anything, not just a single field.
//!
//! A verb is uniform: it is `verb(EntityWorldMut, VerbArgs)`. Mutating a document
//! field is one such behavior, not a structural special case. The verb reads its
//! arguments from [`VerbArgs`]: literal arguments are a plain [`Value`] map
//! ([`VerbArgs::value`]); a binding argument (eg `field: @doc:count`) is a
//! resolved [`BindingArg`] handle ([`VerbArgs::field`]) the verb read-modify-writes
//! against the host through the same ancestor-walk machinery the display bindings
//! use. No synced `Value` mirror is lowered onto the host: the verb writes the
//! real document/resource directly, which document-sync fans out to display
//! bindings, so reactivity is preserved with no per-host state.

use crate::prelude::*;
use alloc::sync::Arc;

/// A parsed `bx:<event>=verb{ arg: value, .. }` binding: DATA only, resolved
/// through the [`EventRegistry`] and [`VerbRegistry`] at build time.
#[derive(Debug, Clone, PartialEq)]
pub struct EventBinding {
	/// The trigger event name, from the `bx:<event>` directive (eg `click`).
	pub event: SmolStr,
	/// The mutation verb name, eg `increment`.
	pub verb: SmolStr,
	/// The named arguments, each a literal value or an `@` binding.
	pub args: Vec<(SmolStr, VerbArg)>,
}

impl EventBinding {
	/// A binding from a [`VerbCall`] under a trigger `event`.
	pub fn new(event: impl Into<SmolStr>, call: VerbCall) -> Self {
		Self {
			event: event.into(),
			verb: call.verb,
			args: call.args,
		}
	}
}

/// A verb handler: a named behavior run with full world access on the event
/// host, reading its named arguments from [`VerbArgs`].
///
/// Exclusive access (an [`EntityWorldMut`]) is deliberate: a verb may read or
/// write beyond the host (eg a document field resolved by ancestor walk, a
/// resource) via [`EntityWorldMut::world_scope`] or a binding argument's
/// read-modify-write helpers, which an inline observer closure cannot express.
/// The event installer queues this to run as an exclusive command, never inline.
pub type VerbFn = Arc<dyn Fn(&mut EntityWorldMut, &VerbArgs) + Send + Sync>;

/// The resolved arguments of a verb invocation: literal arguments as a plain
/// [`Value`] map, plus the resolved binding-argument handles kept separate.
///
/// A literal argument (eg `amount: 3`) is read with [`value`](Self::value); a
/// binding argument (eg `field: @doc:count`) is a [`BindingArg`] read with
/// [`field`](Self::field) and mutated against the host via its helpers. Keeping
/// bindings out of the [`Value`] map is what lets a binding write through to the
/// live source rather than a frozen copy.
#[derive(Debug, Clone, Default)]
pub struct VerbArgs {
	values: Map,
	bindings: HashMap<SmolStr, BindingArg>,
}

impl VerbArgs {
	/// The literal value of argument `name`, if supplied (or defaulted).
	pub fn value(&self, name: &str) -> Option<&Value> { self.values.0.get(name) }

	/// The resolved binding handle of argument `name`, if supplied.
	pub fn field(&self, name: &str) -> Option<&BindingArg> {
		self.bindings.get(name)
	}
}

/// A resolved binding argument: a handle the verb read-modify-writes against the
/// event host, the same source the display bindings sync from.
///
/// A `@doc`/`@prop` argument resolves the document by walking the host's
/// ancestry ([`DocumentQuery`]); a `@res`/`@comp` argument reflect-reads/writes
/// the resource or a component (resolving an `@entity:ref::` target via
/// [`BindingTarget::resolve`]). The verb never sees a mirror [`Value`]: it reads
/// the live source, mutates, and writes back, which document-sync fans out to
/// the display bindings.
#[derive(Debug, Clone)]
pub enum BindingArg {
	/// A `@doc`/`@prop` document field, read-modify-written via [`DocumentQuery`].
	Field(FieldRef),
	/// A `@comp` reflected component field (the `json` bridge), with its target.
	#[cfg(feature = "json")]
	Reflect(ReflectFieldRef),
	/// A `@res` reflected resource field (the `json` bridge).
	#[cfg(feature = "json")]
	Resource(ResourceFieldRef),
}

impl BindingArg {
	/// Read, mutate, and write back the bound source in one step against `host`.
	///
	/// The whole point of the uniform model: the verb mutates the real source
	/// (no per-host mirror), so document-sync fans the change out to display
	/// bindings. A missing document/field is initialized per the binding's
	/// [`FieldRef::on_missing`]; a missing component/resource is an `error!`.
	pub fn update(
		&self,
		host: &mut EntityWorldMut,
		func: impl FnOnce(&mut Value),
	) -> Result {
		match self {
			Self::Field(field) => host
				.with_state::<DocumentQuery, _>(|subject, mut query| {
					query.with_field(subject, field, func)
				})
				.map(|_| ()),
			#[cfg(feature = "json")]
			Self::Reflect(reflect) => {
				let host_id = host.id();
				host.world_scope(|world| {
					component_update(world, host_id, reflect, func)
				});
				Ok(())
			}
			#[cfg(feature = "json")]
			Self::Resource(resource) => {
				host.world_scope(|world| resource_update(world, resource, func));
				Ok(())
			}
		}
	}
}

/// Read-modify-write a reflected component field through a throwaway [`Value`]
/// entity, resolving the target via [`BindingTarget::resolve`] against the host
/// (the same walk the display sync uses). A missing/unregistered component or an
/// unresolved target is an `error!`, naming the component and the `component`
/// verb path.
#[cfg(feature = "json")]
fn component_update(
	world: &mut World,
	host: Entity,
	reflect: &ReflectFieldRef,
	func: impl FnOnce(&mut Value),
) {
	use bevy::ecs::reflect::ReflectComponent;
	let scratch = world.spawn(Value::default()).id();
	let access = reflect_value_ext::field_access(&reflect.field);
	let registry = world.resource::<AppTypeRegistry>().clone();
	let component = &reflect.component;
	let resolved = (|| {
		let reflect_component = registry
			.read()
			.get_with_short_type_path(component)?
			.data::<ReflectComponent>()?
			.clone();
		// `@comp:` (no `$ref`) targets the host; a `$ref`/reserved target resolves
		// from the host's ancestry.
		let target = reflect.target.resolve(world, host)?;
		Some((reflect_component, target))
	})();
	match resolved {
		Some((reflect_component, target)) => {
			reflect_value_ext::read_field_into_value(
				world,
				scratch,
				target,
				&access,
				&reflect_component,
			);
			modify_scratch(world, scratch, func);
			reflect_value_ext::write_field_from_value(
				world,
				scratch,
				target,
				&access,
				&reflect_component,
			);
		}
		None => error!(
			"`component` verb: component `{component}` is not registered, \
			 lacks `#[reflect(Component)]`, or its target could not be resolved"
		),
	}
	world.entity_mut(scratch).despawn();
}

/// Read-modify-write a reflected resource field through a throwaway [`Value`]
/// entity. A missing/unregistered resource is an `error!`, naming the resource
/// and the `resource` verb path.
#[cfg(feature = "json")]
fn resource_update(
	world: &mut World,
	resource: &ResourceFieldRef,
	func: impl FnOnce(&mut Value),
) {
	use bevy::ecs::reflect::ReflectComponent;
	use bevy::ecs::reflect::ReflectResource;
	let scratch = world.spawn(Value::default()).id();
	let access = reflect_value_ext::field_access(&resource.field);
	let registry = world.resource::<AppTypeRegistry>().clone();
	let resource_name = &resource.resource;
	let resolved = (|| {
		let read = registry.read();
		let registration = read.get_with_short_type_path(resource_name)?;
		registration.data::<ReflectResource>()?;
		let reflect_component = registration.data::<ReflectComponent>()?.clone();
		let component_id = world.components().get_id(registration.type_id())?;
		let resource_entity = world.resource_entities().get(component_id)?;
		Some((reflect_component, resource_entity))
	})();
	match resolved {
		Some((reflect_component, resource_entity)) => {
			reflect_value_ext::read_field_into_value(
				world,
				scratch,
				resource_entity,
				&access,
				&reflect_component,
			);
			modify_scratch(world, scratch, func);
			reflect_value_ext::write_field_from_value(
				world,
				scratch,
				resource_entity,
				&access,
				&reflect_component,
			);
		}
		None => error!(
			"`resource` verb: resource `{resource_name}` is not registered, \
			 lacks `#[reflect(Resource)]`, or is not present in the world"
		),
	}
	world.entity_mut(scratch).despawn();
}

/// Run `func` against the scratch entity's [`Value`].
#[cfg(feature = "json")]
fn modify_scratch(
	world: &mut World,
	scratch: Entity,
	func: impl FnOnce(&mut Value),
) {
	if let Some(mut value) = world.entity_mut(scratch).get_mut::<Value>() {
		func(&mut value);
	}
}

/// Records the [`EventBinding`]s installed on an entity so a reactive renderer
/// can re-emit them as `bx:<event>` attributes for the thin client.
///
/// Pure server-side render state: it drives no behavior itself (the installed
/// observers do). The thin client reads the emitted `bx:<event>` attributes and
/// runs the matching JS verb, the browser twin of the native trigger.
#[derive(Debug, Default, Clone, Component)]
pub struct EventBindings(pub Vec<EventBinding>);

/// An event installer: wires the trigger (typically an observer) onto `entity`,
/// running the named verb with its arguments when the trigger fires.
///
/// The installer is where a concrete event type (eg a `PointerDown` observer)
/// and picking live; core never names one. It receives the host entity, the verb
/// name (resolved against the [`VerbRegistry`] at fire time), and the resolved
/// [`VerbArgs`].
pub type EventInstaller =
	Arc<dyn Fn(&mut EntityWorldMut, SmolStr, VerbArgs) + Send + Sync>;

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
		installer: impl Fn(&mut EntityWorldMut, SmolStr, VerbArgs) + Send + Sync + 'static,
	) {
		self.installers.insert(name.into(), Arc::new(installer));
	}

	/// Look up an event installer by name.
	pub fn get(&self, name: &str) -> Option<EventInstaller> {
		self.installers.get(name).cloned()
	}
}

/// A registered verb: its argument [`VerbSchema`] paired with the [`VerbFn`] and,
/// optionally, the source of its JavaScript twin for the thin client.
///
/// The `js_verb` is the body of a `(entity, args) => { .. }` handler (referencing
/// `entity` and `args`), co-located with the Rust verb so registering one without
/// the other is a deliberate choice, not an oversight. The default verb set keeps
/// it `None` (the JS runtime hand-writes those four); an app verb supplies its JS
/// twin here and the reactive renderer emits it into the page. See the
/// `web-reactivity` plan's "hand-written JS twins, no codegen" decision.
#[derive(Clone)]
struct RegisteredVerb {
	schema: VerbSchema,
	verb: VerbFn,
	js_verb: Option<String>,
}

/// The verb seam: verb name -> a [`VerbSchema`] + [`VerbFn`]. Empty by default;
/// an app registers the example verb set (`increment`/`decrement`/`toggle`/`set`).
#[derive(Default, Resource)]
pub struct VerbRegistry {
	verbs: HashMap<SmolStr, RegisteredVerb>,
}

impl VerbRegistry {
	/// Register a verb by name with its argument schema, JavaScript twin, and
	/// handler.
	///
	/// `js_verb` is the body of the matching `(entity, args) => { .. }` thin-client
	/// handler, or `None` for a server-only verb (and for the default set, whose
	/// twins the JS runtime hand-writes). It rides alongside the Rust verb so the
	/// two are registered together.
	pub fn insert(
		&mut self,
		name: impl Into<SmolStr>,
		schema: VerbSchema,
		js_verb: Option<&str>,
		verb: impl Fn(&mut EntityWorldMut, &VerbArgs) + Send + Sync + 'static,
	) {
		self.verbs.insert(
			name.into(),
			RegisteredVerb {
				schema,
				verb: Arc::new(verb),
				js_verb: js_verb.map(Into::into),
			},
		);
	}

	/// The argument schema of a registered verb.
	fn schema(&self, name: &str) -> Option<&VerbSchema> {
		self.verbs.get(name).map(|registered| &registered.schema)
	}

	/// The handler of a registered verb.
	pub fn get(&self, name: &str) -> Option<VerbFn> {
		self.verbs.get(name).map(|registered| registered.verb.clone())
	}

	/// Every registered verb that ships a JavaScript twin, as `(name, js source)`
	/// pairs, for the reactive renderer to emit into the page. Verbs with no
	/// `js_verb` (the default set, server-only verbs) are omitted.
	pub fn js_verbs(&self) -> Vec<(SmolStr, String)> {
		self.verbs
			.iter()
			.filter_map(|(name, registered)| {
				registered
					.js_verb
					.clone()
					.map(|source| (name.clone(), source))
			})
			.collect()
	}
}

/// The schema of a verb's named arguments: which are required, their types, and
/// per-argument defaults, plus which are bindings versus literals.
///
/// Built-time verification ([`verify_against`](Self::verify_against)) rejects an
/// unknown argument, a missing required argument, a literal where a binding is
/// expected (or vice versa), and a type-mismatched literal; an omitted optional
/// literal falls back to its registered default.
#[derive(Debug, Clone, Default)]
pub struct VerbSchema {
	args: Vec<VerbArgSchema>,
}

/// One argument of a [`VerbSchema`].
#[derive(Debug, Clone)]
struct VerbArgSchema {
	name: SmolStr,
	/// Whether the argument is a binding (`@..`) rather than a literal value.
	binding: bool,
	required: bool,
	/// The literal value's schema (unused for a binding argument).
	schema: ValueSchema,
	/// The default literal applied when an optional literal argument is omitted.
	default: Option<Value>,
}

impl VerbSchema {
	/// An empty schema: a verb taking no arguments (a pure side effect).
	pub fn new() -> Self { Self::default() }

	/// A required binding argument (an `@..` source the verb mutates), eg `field`.
	pub fn binding(mut self, name: impl Into<SmolStr>) -> Self {
		self.args.push(VerbArgSchema {
			name: name.into(),
			binding: true,
			required: true,
			schema: ValueSchema::Any,
			default: None,
		});
		self
	}

	/// A required literal argument typed by `schema`, eg `value` of `set`.
	pub fn value(
		mut self,
		name: impl Into<SmolStr>,
		schema: ValueSchema,
	) -> Self {
		self.args.push(VerbArgSchema {
			name: name.into(),
			binding: false,
			required: true,
			schema,
			default: None,
		});
		self
	}

	/// An optional literal argument typed by `schema`, defaulting to `default`
	/// when omitted, eg `amount: i64 = 1`.
	pub fn optional_value(
		mut self,
		name: impl Into<SmolStr>,
		schema: ValueSchema,
		default: impl Into<Value>,
	) -> Self {
		self.args.push(VerbArgSchema {
			name: name.into(),
			binding: false,
			required: false,
			schema,
			default: Some(default.into()),
		});
		self
	}

	/// The schema for argument `name`, if declared.
	fn arg(&self, name: &str) -> Option<&VerbArgSchema> {
		self.args.iter().find(|arg| arg.name == name)
	}

	/// Verify authored `args` against this schema, reporting all failures.
	///
	/// Unknown argument, missing required argument, and a literal/binding kind
	/// mismatch are errors; a literal argument is type-checked against its field
	/// schema. Omitted optional literals are not reported (their default is
	/// applied when the [`VerbArgs`] are built).
	fn verify_against(
		&self,
		verb: &str,
		args: &[(SmolStr, VerbArg)],
	) -> Result<()> {
		let mut errors = Vec::new();
		// each authored argument must be declared, of the right kind, and (for a
		// literal) of the right type.
		for (name, arg) in args {
			let Some(schema) = self.arg(name) else {
				errors.push(format!("unknown argument `{name}`"));
				continue;
			};
			match (schema.binding, arg) {
				(true, VerbArg::Literal(_)) => {
					errors.push(format!("argument `{name}` expects an `@` binding"))
				}
				(false, VerbArg::Binding(_)) => errors
					.push(format!("argument `{name}` expects a literal value")),
				(false, VerbArg::Literal(literal)) => {
					if let Some(message) =
						type_check_literal(&schema.schema, literal)
					{
						errors.push(format!("argument `{name}`: {message}"));
					}
				}
				(true, VerbArg::Binding(_)) => {}
			}
		}
		// every required argument must be supplied.
		for arg in self.args.iter().filter(|arg| arg.required) {
			if !args.iter().any(|(name, _)| name == &arg.name) {
				errors.push(format!("missing required argument `{}`", arg.name));
			}
		}
		if errors.is_empty() {
			Ok(())
		} else {
			bevybail!("verb `{verb}` argument verification failed: {}", errors.join(", "))
		}
	}
}

/// Type-check a literal against a field schema, returning a message on mismatch.
fn type_check_literal(schema: &ValueSchema, literal: &DataLiteral) -> Option<String> {
	let Some(mut value) = literal_value(literal) else {
		return None;
	};
	let errors = async_ext::try_block_on(schema.validate(&mut value)).ok()?;
	errors
		.first()
		.map(|error| error.message.to_string())
}

/// The plain [`Value`] of a literal, or `None` for a non-literal (an entity ref
/// carries no inline value).
///
/// Used both for build-time verification and by the reactive renderer to
/// serialize a literal verb argument into the emitted `bx:<event>` attribute.
pub fn literal_value(literal: &DataLiteral) -> Option<Value> {
	match literal {
		DataLiteral::Scalar(value) => Some(value.clone()),
		DataLiteral::List(items) => items
			.iter()
			.map(literal_value)
			.collect::<Option<Vec<_>>>()
			.map(Value::List),
		DataLiteral::Struct(fields) => {
			let mut map = Map::default();
			for (key, item) in fields {
				map.insert(key.clone(), literal_value(item)?);
			}
			Some(Value::Map(map))
		}
		DataLiteral::Enum(named) if matches!(named.fields, NamedFields::Unit) => {
			Some(Value::Str(named.name.clone()))
		}
		DataLiteral::Enum(_) | DataLiteral::EntityRef(_) => None,
	}
}

/// Resolve a parsed [`VerbArg`] binding to a [`BindingArg`] sink, mirroring the
/// `@`-source dispatch the display bindings use (`apply_binding` in the resolver).
fn resolve_binding_arg(
	binding: &BindingExpr,
	comp_target: BindingTarget,
) -> Result<BindingArg> {
	match binding.source {
		BindingSource::Doc => {
			let mut field = FieldRef::new(binding.field_path.clone());
			if let Some(init) = &binding.init {
				if let Some(value) = literal_value(init) {
					field = field.with_init(value);
				}
			}
			Ok(BindingArg::Field(field))
		}
		BindingSource::Prop => Ok(BindingArg::Field(
			FieldRef::new(binding.field_path.clone())
				.with_document(DocumentPath::Props),
		)),
		#[cfg(feature = "json")]
		BindingSource::Comp => {
			let mut reflect = ReflectFieldRef::new(
				binding.type_path.clone().unwrap_or_default(),
				binding.field_path.to_string(),
			);
			reflect.target = comp_target;
			Ok(BindingArg::Reflect(reflect))
		}
		#[cfg(feature = "json")]
		BindingSource::Res => {
			let _ = comp_target;
			Ok(BindingArg::Resource(ResourceFieldRef::new(
				binding.type_path.clone().unwrap_or_default(),
				binding.field_path.to_string(),
			)))
		}
		#[cfg(not(feature = "json"))]
		BindingSource::Comp | BindingSource::Res => {
			let _ = comp_target;
			bevybail!("`@res`/`@comp` verb arguments require the `json` feature")
		}
	}
}

/// Resolve an [`EventBinding`]'s args against the verb's [`VerbSchema`], building
/// the [`VerbArgs`] (literal values, defaults applied; resolved binding handles).
///
/// `selector_target` resolves a binding argument's `@entity:ref::` selector to its
/// target entity, the element/host by default. Errors if the verb is unknown or
/// an argument fails [`VerbSchema::verify_against`].
fn resolve_args(
	entity: &mut EntityWorldMut,
	binding: &EventBinding,
	selector_target: impl Fn(Option<&SmolStr>) -> BindingTarget,
) -> Result<VerbArgs> {
	let schema = entity.world_scope(|world| {
		world
			.get_resource::<VerbRegistry>()
			.and_then(|registry| registry.schema(&binding.verb).cloned())
	});
	// an unregistered verb has no schema to verify against; build empty args so
	// the loader stays graceful (the trigger no-ops at fire time).
	let Some(schema) = schema else {
		return Ok(VerbArgs::default());
	};
	schema.verify_against(&binding.verb, &binding.args)?;

	let mut args = VerbArgs::default();
	for (name, arg) in &binding.args {
		match arg {
			VerbArg::Literal(literal) => {
				if let Some(value) = literal_value(literal) {
					args.values.insert(name.clone(), value);
				}
			}
			VerbArg::Binding(expr) => {
				let target = selector_target(expr.selector.as_ref());
				args.bindings
					.insert(name.clone(), resolve_binding_arg(expr, target)?);
			}
		}
	}
	// fill omitted optional literals from their registered defaults.
	for arg in &schema.args {
		if let Some(default) = &arg.default
			&& !args.values.0.contains_key(&arg.name)
		{
			args.values.insert(arg.name.clone(), default.clone());
		}
	}
	Ok(args)
}

/// Install an [`EventBinding`] onto `entity`: verify and resolve its arguments,
/// then wire the event's registered trigger to run the verb.
///
/// `selector_target` resolves a binding argument's `@entity:ref::` selector. The
/// trigger is resolved through the [`EventRegistry`]: a registered installer
/// wires it (typically an observer running the named verb from the
/// [`VerbRegistry`] with the resolved [`VerbArgs`]); an unregistered event is a
/// graceful no-op (the loader never fails on an unknown event). Authored
/// arguments that do not match the verb's schema are an error.
pub fn install_event(
	entity: &mut EntityWorldMut,
	binding: &EventBinding,
	selector_target: impl Fn(Option<&SmolStr>) -> BindingTarget,
) -> Result<()> {
	let args = resolve_args(entity, binding, selector_target)?;

	// record the binding so a reactive renderer can re-emit it verbatim as a
	// `bx:<event>` attribute, the thin client's trigger.
	let mut bindings = entity.get::<EventBindings>().cloned().unwrap_or_default();
	bindings.0.push(binding.clone());
	entity.insert(bindings);

	let installer = entity.world_scope(|world| {
		world
			.get_resource::<EventRegistry>()
			.and_then(|registry| registry.get(&binding.event))
	});
	if let Some(installer) = installer {
		installer(entity, binding.verb.clone(), args);
	}
	Ok(())
}

#[cfg(test)]
mod test {
	use super::*;

	/// A `click` installer that runs the verb inline against the host, for tests.
	fn register_inline_click(world: &mut World) {
		world.resource_mut::<EventRegistry>().insert(
			"click",
			|entity: &mut EntityWorldMut, verb: SmolStr, args: VerbArgs| {
				let verb_fn = entity
					.world_scope(|world| world.resource::<VerbRegistry>().get(&verb));
				if let Some(verb_fn) = verb_fn {
					verb_fn(entity, &args);
				}
			},
		);
	}

	#[beet_core::test]
	fn field_helper_writes_document_no_mirror() {
		let mut world = (BsxPlugin, DocumentPlugin).into_world();
		register_inline_click(&mut world);
		// `increment{ field: @doc:count }`: add 1 to the host's ancestor document.
		world.resource_mut::<VerbRegistry>().insert(
			"increment",
			VerbSchema::new().binding("field"),
			None,
			|entity: &mut EntityWorldMut, args: &VerbArgs| {
				if let Some(field) = args.field("field") {
					field
						.update(entity, |value| {
							*value = Value::Int(value.as_i64().unwrap_or(0) + 1)
						})
						.ok();
				}
			},
		);
		let doc = world.spawn(Document::new(val!({ "count": 4 }))).id();
		let binding = EventBinding::new(
			"click",
			VerbCall {
				verb: "increment".into(),
				args: vec![(
					"field".into(),
					VerbArg::Binding(BindingExpr::doc(["count"])),
				)],
			},
		);
		// the inline installer runs the verb during install, against the host.
		let button = {
			let mut entity = world.spawn(ChildOf(doc));
			install_event(&mut entity, &binding, |_| BindingTarget::This).unwrap();
			entity.id()
		};
		world.flush();
		// the host carries NO mirror: no FieldRef, no Value lowered onto it.
		world.entity(button).contains::<FieldRef>().xpect_false();
		world.entity(button).contains::<Value>().xpect_false();
		// the verb wrote the real document.
		world
			.entity(doc)
			.get::<Document>()
			.unwrap()
			.get_field::<i64>(&[FieldSegment::key("count")])
			.unwrap()
			.xpect_eq(5);
	}

	#[beet_core::test]
	fn literal_arg_with_default() {
		let mut world = (BsxPlugin, DocumentPlugin).into_world();
		// `amount` is an optional i64 defaulting to 1.
		let schema =
			VerbSchema::new().binding("field").optional_value(
				"amount",
				ValueSchema::of::<i64>(),
				Value::Int(1),
			);
		// omitted -> default 1
		let omitted = EventBinding::new(
			"click",
			VerbCall {
				verb: "increment".into(),
				args: vec![(
					"field".into(),
					VerbArg::Binding(BindingExpr::doc(["count"])),
				)],
			},
		);
		world.resource_mut::<VerbRegistry>().insert(
			"increment",
			schema,
			None,
			|_, _| {},
		);
		let mut entity = world.spawn_empty();
		let args =
			resolve_args(&mut entity, &omitted, |_| BindingTarget::This).unwrap();
		args.value("amount")
			.and_then(|value| value.as_i64().ok())
			.unwrap_or(0)
			.xpect_eq(1);
	}

	#[beet_core::test]
	fn side_effect_verb_no_binding() {
		let mut world = (BsxPlugin, DocumentPlugin).into_world();
		register_inline_click(&mut world);
		// a verb with zero binding args: a pure side effect (inserts a marker).
		world.resource_mut::<VerbRegistry>().insert(
			"mark",
			VerbSchema::new(),
			None,
			|entity: &mut EntityWorldMut, _: &VerbArgs| {
				entity.insert(DocumentScope {
					path: FieldPath::new(["marked"]),
					terminate: false,
				});
			},
		);
		let binding = EventBinding::new(
			"click",
			VerbCall {
				verb: "mark".into(),
				args: Vec::new(),
			},
		);
		// the inline installer runs the side-effect verb during install.
		let button = {
			let mut entity = world.spawn_empty();
			install_event(&mut entity, &binding, |_| BindingTarget::This).unwrap();
			entity.id()
		};
		world.flush();
		world.entity(button).contains::<DocumentScope>().xpect_true();
	}

	#[beet_core::test]
	fn verify_rejects_unknown_missing_and_mistyped() {
		// verification is pure: no world needed.
		let schema = VerbSchema::new()
			.binding("field")
			.optional_value("amount", ValueSchema::of::<i64>(), Value::Int(1));

		// unknown argument
		schema
			.verify_against(
				"increment",
				&[
					("field".into(), VerbArg::Binding(BindingExpr::doc(["c"]))),
					(
						"bogus".into(),
						VerbArg::Literal(DataLiteral::Scalar(Value::Int(1))),
					),
				],
			)
			.unwrap_err()
			.to_string()
			.xpect_contains("unknown argument `bogus`");

		// missing required binding
		schema
			.verify_against("increment", &[])
			.unwrap_err()
			.to_string()
			.xpect_contains("missing required argument `field`");

		// type mismatch on a literal
		schema
			.verify_against(
				"increment",
				&[
					("field".into(), VerbArg::Binding(BindingExpr::doc(["c"]))),
					(
						"amount".into(),
						VerbArg::Literal(DataLiteral::Scalar(Value::Str("x".into()))),
					),
				],
			)
			.unwrap_err()
			.to_string()
			.xpect_contains("amount");

		// a literal where a binding is expected
		schema
			.verify_against(
				"increment",
				&[(
					"field".into(),
					VerbArg::Literal(DataLiteral::Scalar(Value::Int(1))),
				)],
			)
			.unwrap_err()
			.to_string()
			.xpect_contains("expects an `@` binding");
	}

	/// The `@comp`/`@res` field helpers (json-gated) read-modify-write a
	/// reflected component/resource field with no mirror on the host.
	#[cfg(feature = "json")]
	#[beet_core::test]
	fn field_helper_writes_component_and_resource() {
		use bevy::ecs::reflect::ReflectResource;

		#[derive(Component, Reflect, Default, Clone, PartialEq, Debug)]
		#[reflect(Component, Default)]
		struct Counter {
			value: i64,
		}
		#[derive(Resource, Reflect, Default, Clone, PartialEq, Debug)]
		#[reflect(Resource, Default)]
		struct Score {
			points: i64,
		}

		let mut world = (BsxPlugin, DocumentPlugin).into_world();
		{
			let registry = world.resource_mut::<AppTypeRegistry>();
			let mut registry = registry.write();
			registry.register::<Counter>();
			registry.register::<Score>();
		}
		world.insert_resource(Score { points: 10 });

		// `@comp:Counter.value` on the host: bump the host's own component field.
		let host = world.spawn(Counter { value: 3 }).id();
		BindingArg::Reflect(ReflectFieldRef::new("Counter", "value"))
			.update(&mut world.entity_mut(host), |value| {
				*value = Value::Int(value.as_i64().unwrap_or(0) + 1)
			})
			.unwrap();
		world.entity(host).get::<Counter>().unwrap().value.xpect_eq(4);

		// `@res:Score.points`: bump the resource field, no mirror entity.
		BindingArg::Resource(ResourceFieldRef::new("Score", "points"))
			.update(&mut world.entity_mut(host), |value| {
				*value = Value::Int(value.as_i64().unwrap_or(0) + 5)
			})
			.unwrap();
		world.resource::<Score>().points.xpect_eq(15);
	}
}
