//! The by-name template registry bridge.
//!
//! Bevy resolves a `Template` by its concrete Rust type. beet additionally
//! needs to resolve one by name (a short type path from markup or a serialized
//! tag), so it owns [`ReflectTemplate`]: reflect type-data that builds a
//! template from a reflected data value. `app.install_type::<T>()` installs it, and
//! [`ReflectTemplate::build_named`] looks it up from the [`AppTypeRegistry`].
//!
//! A schema attaches alongside this registration: the type-data stays a thin
//! build bridge, and schemas register beside it rather than inside it.

use crate::prelude::*;
use bevy::ecs::template::Template;
use bevy::ecs::template::TemplateContext;
use bevy::reflect::CreateTypeData;
use bevy::reflect::GetTypeRegistration;
use bevy::reflect::TypeRegistration;
use bevy::reflect::TypeRegistry;
use bevy::reflect::Typed;
use core::any::TypeId;

/// Reflect type-data that builds a template from a reflected data value.
///
/// Stored against a registered template type, keyed by its short type path, so a
/// tag resolves to a build function without the concrete type in hand. The
/// `#[derive(Clone)]` gives it the blanket [`TypeData`](bevy::reflect::TypeData)
/// impl, so it lives in the registry.
///
/// Registering it also inserts the schema-side companion
/// [`ReflectTemplateSchema`] beside it, keeping prop validation separate from
/// the build bridge.
#[derive(Clone)]
pub struct ReflectTemplate {
	/// Builds the template: `from_reflect` the data into `T`, then
	/// `build_template` it into the context.
	build: fn(&dyn PartialReflect, &mut TemplateContext) -> Result<()>,
}

impl ReflectTemplate {
	/// The registration under type path `name`, full or short, resolving an
	/// ambiguous short path in favour of a sole template candidate.
	///
	/// A `#[template]` type's short path is its *only* name: it exists to be
	/// authored as `<Button>` and is reachable no other way, while a component
	/// or resource is equally reachable from Rust and, from markup, by its full
	/// type path. So a template wins a collision with foreign types of the same
	/// name (beet_ui's `<Button>` widget over bevy_ui's `Button` component),
	/// while a collision no template is party to (the two registered
	/// `Transform`s) still resolves to nothing rather than guessing.
	pub fn registration_named<'a>(
		registry: &'a TypeRegistry,
		name: &str,
	) -> Option<&'a TypeRegistration> {
		if let Some(registration) = registry
			.get_with_type_path(name)
			.or_else(|| registry.get_with_short_type_path(name))
		{
			return Some(registration);
		}
		if !registry.is_ambiguous(name) {
			return None;
		}
		let mut templates = registry.iter().filter(|registration| {
			registration.type_info().type_path_table().short_path() == name
				&& registration.data::<ReflectTemplate>().is_some()
		});
		let template = templates.next()?;
		templates.next().is_none().then_some(template)
	}

	/// Builds the template from a reflected `value` into `cx`.
	///
	/// `value` is the template's data (eg a `DynamicStruct` patch). It is
	/// `from_reflect`ed into the concrete template type, then built.
	pub fn build(
		&self,
		value: &dyn PartialReflect,
		cx: &mut TemplateContext,
	) -> Result {
		(self.build)(value, cx)
	}

	/// Builds the template registered under short type path `tag` from `value`
	/// into `cx`, the build-side companion of [`ValueSchema::template_by_name`].
	/// Errors if no template is registered under that tag.
	pub fn build_named(
		registry: &AppTypeRegistry,
		tag: &str,
		value: &dyn PartialReflect,
		cx: &mut TemplateContext,
	) -> Result {
		let guard = registry.read();
		let registration =
			Self::registration_named(&guard, tag).ok_or_else(|| {
				bevyhow!("no type registered for template tag `{tag}`")
			})?;
		let reflect_template =
			registration.data::<ReflectTemplate>().ok_or_else(|| {
				bevyhow!("type `{tag}` is registered but is not a template")
			})?;
		reflect_template.build(value, cx)
	}
}

/// Reflect type-data carrying a template type's prop [`ValueSchema`], inserted
/// alongside every [`ReflectTemplate`] registration as its dependency.
///
/// The schema sits *beside* the build bridge, not inside it: a tag resolves to
/// its build function (`ReflectTemplate`) and, separately,
/// to its prop schema (this), so the loader can verify props against the schema
/// without building. A `<Foo a=.. b=../>` prop set is validated against the
/// struct schema, surfacing a missing required field as a graceful error.
///
/// The default schema is derived from `T`'s reflect [`TypeInfo`] via
/// [`ValueSchema::of`]; an `Option<T>` prop field unwraps to an optional inner
/// schema. The `#[template]` macro overrides it with a precise schema that marks
/// `#[prop(required)]` props as required (which the type alone cannot express).
#[derive(Clone)]
pub struct ReflectTemplateSchema {
	/// The template's prop schema.
	pub schema: ValueSchema,
}

impl ReflectTemplateSchema {
	/// The template's prop [`ValueSchema`], the props it accepts and their types.
	/// Read by tooling (eg the diagnostics manifest) to enumerate a tag's props.
	pub fn schema(&self) -> &ValueSchema { &self.schema }
}

impl<T> CreateTypeData<T> for ReflectTemplateSchema
where
	T: GetTemplateSchema,
{
	fn create_type_data(_input: ()) -> Self {
		Self {
			schema: T::template_schema(),
		}
	}
}

/// A template type's prop [`ValueSchema`], the schema-side companion of its build.
///
/// A `#[template]` implements this from its typed signature, marking
/// `#[prop(required)]` props as required (which a reflect-derived schema cannot
/// express, since a required prop is stored as an `Option`). A
/// hand-written template that wants no prop validation takes the default
/// [`ValueSchema::Any`].
pub trait GetTemplateSchema {
	/// The template's prop schema. Defaults to a wildcard (no validation).
	fn template_schema() -> ValueSchema { ValueSchema::Any }
}

impl<T> CreateTypeData<T> for ReflectTemplate
where
	T: FromReflect + Typed + GetTemplateSchema + Template<Output = ()>,
{
	fn create_type_data(_input: ()) -> Self {
		Self {
			build: |value, cx| {
				let template = T::from_reflect(value).ok_or_else(|| {
					bevyhow!(
						"failed to build template `{}` from reflected value",
						TypeName::of::<T>()
					)
				})?;
				template.build_template(cx)
			},
		}
	}

	/// A build bridge always carries its prop schema beside it.
	fn insert_dependencies(registration: &mut TypeRegistration) {
		registration.register_type_data::<ReflectTemplateSchema, T>();
	}
}

/// The [`TypeInstaller`] marker for a template.
pub struct TemplateMarker;

/// A template installs by type, `app.install_type::<Route>()`: its type and
/// [`ReflectTemplate`] type-data, which brings its [`ReflectTemplateSchema`]
/// along, so it resolves by short type path, and its schema in the
/// [`SchemaRegistry`].
impl<T> TypeInstaller<TemplateMarker> for T
where
	T: FromReflect
		+ Typed
		+ GetTypeRegistration
		+ GetTemplateSchema
		+ Template<Output = ()>,
{
	fn install(app: &mut App) {
		let world = app.world_mut();
		let registry = world.resource_mut::<AppTypeRegistry>();
		let mut registry = registry.write();
		registry.register::<T>();
		// `TypeRegistry::register_type_data` skips dependencies, so insert
		// through the registration to bring the schema along.
		registry
			.get_mut(TypeId::of::<T>())
			.expect("registered above")
			.register_type_data::<ReflectTemplate, T>();
		drop(registry);
		// index the schema by short type path so a `SchemaRef::Name` resolves.
		let name = T::type_info().type_path_table().short_path();
		world
			.get_resource_or_init::<SchemaRegistry>()
			.insert(SmolStr::from(name), T::template_schema());
	}
}

/// The components a loader could not resolve on an entity, held as values
/// keyed by the name their source wrote (`TofuApply` from BSX,
/// `beet_infra::TofuApply` from a scene body), so the entity and its children
/// exist in every binary and only behavior is missing. Never saved as itself:
/// a saver writes each held component back as one of the entity's own keys,
/// so a binary with the plugin reads it on the next load and a round trip
/// through a lean one loses nothing. `beet check`, which registers
/// everything, elevates every one.
///
/// Structure is universal: a document builds the same entity tree in every
/// binary, and what a lean build lacks is behavior (unlinked components, absent
/// actions), never structure. So an unresolvable uppercase tag or spread, and
/// a scene body's key naming a type this binary never registered, each warn,
/// are held here, and leave the entity's directives, spreads and children to
/// build, which keeps `bx:ref` targets and the shape of the tree stable
/// across builds.
///
/// Loudness arrives later, where it can distinguish a lean binary from a typo:
/// a subtree excluded by `bx:cfg` reports its condition rather than the missing
/// features, a failed sequence route appends its unregistered children's names
/// at the reporting edge, and a strict caller refuses through
/// [`UnregisteredQuery`].
#[derive(
	Debug, Default, Clone, PartialEq, Deref, DerefMut, Component, Reflect,
)]
#[reflect(Component, Default)]
pub struct Unregistered(EntityBody);

/// A marker holding each `(name, value)`, in order.
impl<K: Into<SmolStr>, V: Into<Value>> FromIterator<(K, V)> for Unregistered {
	fn from_iter<I: IntoIterator<Item = (K, V)>>(entries: I) -> Self {
		Self(Map::new(entries).into())
	}
}

impl Unregistered {
	/// A marker holding `value` under `name`.
	pub fn new(name: impl Into<SmolStr>, value: impl Into<Value>) -> Self {
		let mut unregistered = Self::default();
		unregistered.insert(name, value);
		unregistered
	}

	/// Insert `value` under `name` into `entity`'s marker, adding the marker
	/// when it has none.
	pub fn insert_on(
		entity: &mut EntityWorldMut,
		name: impl Into<SmolStr>,
		value: impl Into<Value>,
	) {
		match entity.get_mut::<Self>() {
			Some(mut unregistered) => {
				unregistered.insert(name, value);
			}
			None => {
				entity.insert(Self::new(name, value));
			}
		}
	}

	/// Remove `name` from `entity`'s marker, and the marker with its last
	/// entry.
	pub fn remove_from(entity: &mut EntityWorldMut, name: &str) {
		let Some(mut unregistered) = entity.get_mut::<Self>() else {
			return;
		};
		unregistered.remove(name);
		if unregistered.is_empty() {
			entity.remove::<Self>();
		}
	}
}

/// Every [`Unregistered`] under a root: what `beet check`, a failed
/// sequence's report and a strict caller read.
#[derive(SystemParam)]
pub struct UnregisteredQuery<'w, 's> {
	unregistered: Query<'w, 's, &'static Unregistered>,
	children: Query<'w, 's, &'static Children>,
	attributes: Query<'w, 's, &'static Attributes>,
}

impl UnregisteredQuery<'_, '_> {
	/// Every marked entity under `root`, inclusive, in tree order: an entity,
	/// then its attributes, then its children.
	pub fn collect(&self, root: Entity) -> Vec<(Entity, &Unregistered)> {
		let mut found = Vec::new();
		let mut stack = vec![root];
		while let Some(entity) = stack.pop() {
			if let Ok(unregistered) = self.unregistered.get(entity) {
				found.push((entity, unregistered));
			}
			let owned = self
				.attributes
				.get(entity)
				.into_iter()
				.flat_map(|attributes| attributes.iter())
				.chain(
					self.children
						.get(entity)
						.into_iter()
						.flat_map(|children| children.iter()),
				)
				.collect::<Vec<_>>();
			stack.extend(owned.into_iter().rev());
		}
		found
	}

	/// Whether `entity` itself holds an unresolved component.
	pub fn contains(&self, entity: Entity) -> bool {
		self.unregistered.contains(entity)
	}

	/// The name of every unresolved component under `root`, in tree order.
	pub fn names(&self, root: Entity) -> Vec<&SmolStr> {
		self.collect(root)
			.into_iter()
			.flat_map(|(_, unregistered)| unregistered.keys())
			.collect()
	}

	/// An error naming every unresolved component under `root`, `Ok` when none.
	pub fn refuse(&self, root: Entity) -> Result {
		let names = self.names(root);
		match names.is_empty() {
			true => OK,
			false => bevybail!(
				"unregistered in this binary: {}",
				names
					.iter()
					.map(|name| format!("`{name}`"))
					.collect::<Vec<_>>()
					.join(", ")
			),
		}
	}
}

impl ValueSchema {
	/// The prop [`ValueSchema`] registered for the template under short type path
	/// `tag`, if any. The schema-side companion of [`ReflectTemplate::build_named`].
	pub fn template_by_name(
		registry: &AppTypeRegistry,
		tag: &str,
	) -> Option<ValueSchema> {
		let registry = registry.read();
		ReflectTemplate::registration_named(&registry, tag)
			.and_then(|registration| {
				registration.data::<ReflectTemplateSchema>()
			})
			.map(|data| data.schema.clone())
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use bevy::reflect::structs::DynamicStruct;

	/// A template registered by name that spawns a child labelled by its field.
	#[derive(Default, Clone, Reflect)]
	#[reflect(Default)]
	struct Label {
		text: String,
	}

	// opt out of `Unpin` so `Label` escapes Bevy's blanket
	// `Template for T: Clone + Unpin` and can supply its own
	// subtree-building impl. See [`subtree_template`].
	subtree_template!(Label);
	impl GetTemplateSchema for Label {}

	impl Template for Label {
		type Output = ();
		fn build_template(&self, cx: &mut TemplateContext) -> Result<()> {
			let text = self.text.clone();
			let root = cx.entity.id();
			// SAFETY: only used to spawn an unrelated child entity.
			let world = unsafe { cx.entity.world_mut() };
			world.spawn((Name::new(text), ChildOf(root)));
			OK
		}
		fn clone_template(&self) -> Self { self.clone() }
	}

	#[crate::test]
	fn builds_by_name() {
		let mut app = App::new();
		app.add_plugins(TemplatePlugin).install_type::<Label>();
		let world = app.world_mut();

		// a partial patch over default, the loader's form.
		let mut patch = DynamicStruct::default();
		patch.insert("text", "hello".to_string());

		let registry = world.resource::<AppTypeRegistry>().clone();
		// the build bridge brings its prop schema along
		ValueSchema::template_by_name(&registry, "Label").xpect_some();
		let root = world.spawn_empty().id();
		world
			.entity_mut(root)
			.template_context(|cx| {
				ReflectTemplate::build_named(&registry, "Label", &patch, cx)
			})
			.unwrap();

		let kid = world.entity(root).get::<Children>().unwrap()[0];
		world
			.entity(kid)
			.get::<Name>()
			.unwrap()
			.as_str()
			.xpect_eq("hello");
	}

	#[crate::test]
	fn unregistered_tag_errors() {
		let world = TemplatePlugin::world();
		let registry = world.resource::<AppTypeRegistry>().clone();
		let patch = DynamicStruct::default();
		let mut other = World::new();
		let root = other.spawn_empty().id();
		other
			.entity_mut(root)
			.template_context(|cx| {
				ReflectTemplate::build_named(&registry, "Nope", &patch, cx)
			})
			.unwrap_err();
	}

	/// Types sharing a short path with the test's own, the bevy-widget collision
	/// shape: `Label` against the registered template, `Sphere` against a second
	/// non-template.
	mod foreign {
		use crate::prelude::*;

		#[derive(Default, Clone, Reflect)]
		#[reflect(Default)]
		pub struct Label;

		#[derive(Default, Clone, Reflect)]
		#[reflect(Default)]
		pub struct Sphere;
	}
	mod other {
		use crate::prelude::*;

		#[derive(Default, Clone, Reflect)]
		#[reflect(Default)]
		pub struct Sphere;
	}

	/// A template's short path is its only name, so it wins an ambiguous short
	/// path; an ambiguity no template is party to still resolves to nothing
	/// rather than guessing, and is named by its full type path instead.
	#[crate::test]
	fn a_template_wins_an_ambiguous_short_path() {
		let mut app = App::new();
		app.add_plugins(TemplatePlugin).install_type::<Label>();
		let world = app.world_mut();
		let app_registry = world.resource::<AppTypeRegistry>().clone();
		{
			let mut registry = app_registry.write();
			registry.register::<foreign::Label>();
			registry.register::<foreign::Sphere>();
			registry.register::<other::Sphere>();
		}
		let registry = app_registry.read();
		// bevy's own lookup drops an ambiguous short path entirely..
		registry
			.get_with_short_type_path("Label")
			.is_none()
			.xpect_true();
		// ..and the sole template candidate wins it.
		ReflectTemplate::registration_named(&registry, "Label")
			.unwrap()
			.type_info()
			.type_path()
			.xpect_eq(<Label as Typed>::type_info().type_path());
		// two non-templates stay ambiguous.
		ReflectTemplate::registration_named(&registry, "Sphere")
			.is_none()
			.xpect_true();
	}

	/// The query walks an entity, its attributes, then its children, and a
	/// strict caller's refusal names every held component in that order.
	#[crate::test]
	fn refuses_naming_everything_under_a_root() {
		let mut world = TemplatePlugin::world();
		let root = world
			.spawn((Unregistered::new("Rooty", Value::map()), children![
				(Unregistered::new("my_crate::First", 1u64), children![
					Unregistered::new("Deep", Value::Null)
				]),
				Unregistered::new("Second", Value::Null),
			]))
			.flush();
		let attribute = world
			.spawn((
				Attribute::new("class"),
				AttributeOf::new(root),
				Unregistered::new("OnAttribute", Value::Null),
			))
			.id();
		let outside = world.spawn(Unregistered::new("Outside", 0u64)).id();

		world.with_state::<UnregisteredQuery, _>(|query| {
			query
				.names(root)
				.into_iter()
				.map(SmolStr::as_str)
				.collect::<Vec<_>>()
				.xpect_eq(vec![
					"Rooty",
					"OnAttribute",
					"my_crate::First",
					"Deep",
					"Second",
				]);
			query.collect(root)[1].0.xpect_eq(attribute);
			query
				.refuse(root)
				.unwrap_err()
				.to_string()
				.xpect_contains("`Rooty`, `OnAttribute`, `my_crate::First`");
			query.refuse(outside).unwrap_err();
		});
		// removing the last entry drops the marker
		Unregistered::remove_from(&mut world.entity_mut(outside), "Outside");
		world.get::<Unregistered>(outside).xpect_none();
		world
			.with_state::<UnregisteredQuery, _>(|query| query.refuse(outside))
			.unwrap();
	}
}
