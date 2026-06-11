//! The by-name template registry bridge.
//!
//! Bevy resolves a `Template` by its concrete Rust type. beet additionally
//! needs to resolve one by name (a short type path from markup or a serialized
//! tag), so it owns [`ReflectTemplate`]: reflect type-data that builds a
//! template from a reflected data value. [`register_template`] installs it, and
//! [`build_template_by_name`] looks it up from the [`AppTypeRegistry`].
//!
//! A schema attaches alongside this registration: the type-data stays a thin
//! build bridge, and schemas register beside it rather than inside it.

use crate::prelude::*;
use bevy::ecs::template::Template;
use bevy::ecs::template::TemplateContext;
use bevy::reflect::FromType;
use bevy::reflect::GetTypeRegistration;
use bevy::reflect::Typed;

/// Reflect type-data that builds a template from a reflected data value.
///
/// Stored against a registered template type, keyed by its short type path, so a
/// tag resolves to a build function without the concrete type in hand. The
/// `#[derive(Clone)]` gives it the blanket [`TypeData`](bevy::reflect::TypeData)
/// impl, so it lives in the registry.
///
/// The schema-side companion [`ReflectTemplateSchema`] registers beside this,
/// keeping prop validation separate from the build bridge.
#[derive(Clone)]
pub struct ReflectTemplate {
	/// Builds the template: `from_reflect` the data into `T`, then
	/// `build_template` it into the context.
	build: fn(&dyn PartialReflect, &mut TemplateContext) -> Result<()>,
}

impl ReflectTemplate {
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
}

/// Reflect type-data carrying a template type's prop [`ValueSchema`], attached
/// alongside [`ReflectTemplate`] by [`register_template`](WorldRegisterTemplateExt::register_template).
///
/// The schema sits *beside* the build bridge, not inside it: a tag resolves to
/// its build function (`ReflectTemplate`) and, separately,
/// to its prop schema (this), so the loader can verify props against the schema
/// without building. A `<Foo a=.. b=../>` prop set is validated against the
/// struct schema, surfacing a missing required field as a graceful error.
///
/// The default schema is derived from `T`'s reflect [`TypeInfo`] via
/// [`ValueSchema::of`]; a `PropOpt<T>` prop field unwraps to an optional inner
/// schema. The `#[template]` macro overrides it with a precise schema that marks
/// `#[prop(required)]` props as required (which the type alone cannot express).
#[derive(Clone)]
pub(crate) struct ReflectTemplateSchema {
	/// The template's prop schema.
	pub schema: ValueSchema,
}

impl<T> FromType<T> for ReflectTemplateSchema
where
	T: GetTemplateSchema,
{
	fn from_type() -> Self {
		Self {
			schema: T::template_schema(),
		}
	}
}

/// A template type's prop [`ValueSchema`], the schema-side companion of its build.
///
/// A `#[template]` implements this from its typed signature, marking
/// `#[prop(required)]` props as required (which a reflect-derived schema cannot
/// express, since a required prop is stored as an optional `PropOpt`). A
/// hand-written template that wants no prop validation takes the default
/// [`ValueSchema::Any`].
pub trait GetTemplateSchema {
	/// The template's prop schema. Defaults to a wildcard (no validation).
	fn template_schema() -> ValueSchema { ValueSchema::Any }
}

impl<T> FromType<T> for ReflectTemplate
where
	T: FromReflect + Typed + Template<Output = ()>,
{
	fn from_type() -> Self {
		Self {
			build: |value, cx| {
				let template = T::from_reflect(value).ok_or_else(|| {
					bevyhow!(
						"failed to build template `{}` from reflected value",
						core::any::type_name::<T>()
					)
				})?;
				template.build_template(cx)
			},
		}
	}
}

/// Registers a template type on a [`World`].
#[extend::ext(name=WorldRegisterTemplateExt)]
pub impl World {
	/// Registers `T` and its [`ReflectTemplate`] type-data, so it resolves by
	/// short type path.
	fn register_template<T>(&mut self) -> &mut Self
	where
		T: FromReflect
			+ Typed
			+ GetTypeRegistration
			+ GetTemplateSchema
			+ Template<Output = ()>,
	{
		let registry = self.resource_mut::<AppTypeRegistry>();
		let mut registry = registry.write();
		registry.register::<T>();
		registry.register_type_data::<T, ReflectTemplate>();
		// attach the prop schema (from `GetTemplateSchema`) beside the build bridge.
		registry.register_type_data::<T, ReflectTemplateSchema>();
		drop(registry);
		// index the schema by short type path so a `ValueSchema::Reference` resolves.
		let name = T::type_info().type_path_table().short_path();
		self.get_resource_or_init::<SchemaRegistry>()
			.insert(SmolStr::from(name), T::template_schema());
		self
	}
}

/// Registers a template type on an [`App`].
#[extend::ext(name=AppRegisterTemplateExt)]
pub impl App {
	/// Registers `T` and its [`ReflectTemplate`] type-data. See
	/// [`WorldRegisterTemplateExt::register_template`].
	fn register_template<T>(&mut self) -> &mut Self
	where
		T: FromReflect
			+ Typed
			+ GetTypeRegistration
			+ GetTemplateSchema
			+ Template<Output = ()>,
	{
		self.world_mut().register_template::<T>();
		self
	}
}

/// The prop [`ValueSchema`] registered for the template under short type path
/// `tag`, if any. The schema-side companion of [`build_template_by_name`].
pub fn template_schema_by_name(
	registry: &AppTypeRegistry,
	tag: &str,
) -> Option<ValueSchema> {
	let registry = registry.read();
	registry
		.get_with_short_type_path(tag)
		.and_then(|registration| registration.data::<ReflectTemplateSchema>())
		.map(|data| data.schema.clone())
}

/// Builds a registered template by its short type path into `cx`.
///
/// Resolves `tag` to a [`ReflectTemplate`] from `registry`, then builds it from
/// `value`. Errors if no template is registered under that tag.
pub fn build_template_by_name(
	registry: &AppTypeRegistry,
	tag: &str,
	value: &dyn PartialReflect,
	cx: &mut TemplateContext,
) -> Result {
	let registry = registry.read();
	let registration = registry
		.get_with_short_type_path(tag)
		.ok_or_else(|| bevyhow!("no type registered for template tag `{tag}`"))?;
	let reflect_template =
		registration.data::<ReflectTemplate>().ok_or_else(|| {
			bevyhow!("type `{tag}` is registered but is not a template")
		})?;
	reflect_template.build(value, cx)
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
	// `Template for T: Default + Clone + Unpin` and can supply its own
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

	#[beet_core::test]
	fn builds_by_name() {
		let mut world = TemplatePlugin::world();
		world.register_template::<Label>();

		// a partial patch over default, the loader's form.
		let mut patch = DynamicStruct::default();
		patch.insert("text", "hello".to_string());

		let registry = world.resource::<AppTypeRegistry>().clone();
		let root = world.spawn_empty().id();
		world
			.entity_mut(root)
			.template_context(|cx| {
				build_template_by_name(&registry, "Label", &patch, cx)
			})
			.unwrap();

		let kid = world.entity(root).get::<Children>().unwrap()[0];
		world.entity(kid).get::<Name>().unwrap().as_str().xpect_eq("hello");
	}

	#[beet_core::test]
	fn unregistered_tag_errors() {
		let world = TemplatePlugin::world();
		let registry = world.resource::<AppTypeRegistry>().clone();
		let patch = DynamicStruct::default();
		let mut other = World::new();
		let root = other.spawn_empty().id();
		other
			.entity_mut(root)
			.template_context(|cx| {
				build_template_by_name(&registry, "Nope", &patch, cx)
			})
			.unwrap_err();
	}
}
