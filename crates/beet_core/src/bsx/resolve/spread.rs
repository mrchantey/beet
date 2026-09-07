//! Bare-position spreads: resolving a spread's components and templates onto
//! the entity, and the reflect patch each named type is built from.

use super::binding::*;
use super::directives::*;
use super::entity_refs::*;
use super::literal::*;
use super::uppercase::*;
use crate::bsx::reflect::*;
use crate::prelude::*;
use bevy::ecs::template::TemplateContext;
use bevy::reflect::TypeRegistry;

/// Insert every bare-position spread's components/templates onto `entity`,
/// shared by every tag kind (an HTML element, a component, a template). The
/// `AppTypeRegistry` is only touched when a spread is present, so a plain
/// HTML/markdown parse (no registry) never needs it.
pub(super) fn apply_spreads(
	el: &BsxElement,
	entity: &mut EntityWorldMut,
	entity_refs: &HashMap<SmolStr, Entity>,
) -> Result<()> {
	let has_spread = el
		.attributes
		.iter()
		.any(|attr| matches!(attr.value, AttrValue::Spread(_)));
	if !has_spread {
		return Ok(());
	}
	let app_registry = entity
		.world_scope(|world| world.get_resource::<AppTypeRegistry>().cloned())
		.ok_or_else(|| {
			bevyhow!("a spread requires an `AppTypeRegistry` in the world")
		})?;
	for attr in &el.attributes {
		if let AttrValue::Spread(spread) = &attr.value {
			apply_spread(spread, entity, &app_registry, entity_refs)?;
		}
	}
	Ok(())
}

/// Insert or build a spread's components/templates onto `entity`. A tuple's
/// `@` binding items apply to the same entity, pairing a component insert with
/// its binding, eg `{(Bar{boo:"bazz"}, @comp:Bar.boo)}`.
fn apply_spread(
	spread: &SpreadExpr,
	entity: &mut EntityWorldMut,
	app_registry: &AppTypeRegistry,
	entity_refs: &HashMap<SmolStr, Entity>,
) -> Result<()> {
	match spread {
		SpreadExpr::Named(named) => {
			apply_spread_named(named, entity, app_registry, entity_refs)
		}
		SpreadExpr::Tuple(items) => {
			for item in items {
				match item {
					SpreadItem::Named(named) => apply_spread_named(
						named,
						entity,
						app_registry,
						entity_refs,
					)?,
					// spread position: an `@comp` binds this entity unless
					// `$ref` retargets.
					SpreadItem::Binding(binding) => {
						let comp_target = match &binding.selector {
							Some(name) => {
								map_selector_target(name, entity_refs)
							}
							None => BindingTarget::This,
						};
						apply_binding(binding, entity, comp_target)?;
					}
				}
			}
			Ok(())
		}
	}
}

/// Insert or build one named spread component/template onto `entity`.
///
/// A name with no registered type is a capability this binary did not link (eg a
/// `<Router {(.., TuiServer)}>` spread loaded by a lean http-only deploy that
/// dropped the `tui` feature). Skip it with a warning rather than failing the
/// whole load, so the same site serves the subset each binary supports.
pub(super) fn apply_spread_named(
	named: &NamedLiteral,
	entity: &mut EntityWorldMut,
	app_registry: &AppTypeRegistry,
	entity_refs: &HashMap<SmolStr, Entity>,
) -> Result<()> {
	let literal = DataLiteral::Enum(named.clone());
	let (kind, patch) = {
		let registry = app_registry.read();
		// resolve by base name so a generic spread (eg `{Repeat}` -> `Repeat<()>`)
		// matches its sole instantiation, exactly as a `<Repeat>` tag does. A
		// `::`-qualified name may be an enum variant (`{SteerTarget::Entity($x)}`):
		// fall back to the enum type so the variant resolves through the literal.
		let Some(registration) = registration_by_name(&registry, &named.name)
			.or_else(|| enum_variant_registration(&registry, &named.name))
		else {
			drop(registry);
			if inertness_declared(entity) {
				debug!(
					"skipping spread `{}`: not registered in this binary (declared by `RequireFeatures`)",
					named.name
				);
			} else {
				warn!(
					"skipping spread `{}`: no component or template of that name is registered in this binary",
					named.name
				);
			}
			return Ok(());
		};
		let info = Some(registration.type_info());
		// classify the spread type like an uppercase tag: a `#[template]` builds,
		// a `#[reflect(Resource)]` writes the resource, the rest insert as a component.
		let kind = if registration.data::<ReflectTemplate>().is_some() {
			UppercaseKind::Template
		} else if registration
			.data::<bevy::ecs::reflect::ReflectResource>()
			.is_some()
		{
			UppercaseKind::Resource
		} else {
			UppercaseKind::Component
		};
		let mut resolver = entity_ref_resolver(entity_refs);
		(
			kind,
			DataLiteral::to_reflect(&literal, info, &registry, &mut resolver)?,
		)
	};
	match kind {
		UppercaseKind::Template => {
			let id = entity.id();
			entity.world_scope(|world| -> Result<()> {
				let mut references =
					bevy::ecs::template::SceneEntityReferences::default();
				let mut entity_mut = world.entity_mut(id);
				let mut cx =
					TemplateContext::new(&mut entity_mut, &mut references);
				build_template_by_name(
					app_registry,
					&named.name,
					patch.as_ref(),
					&mut cx,
				)
			})?;
		}
		// a spread resource patches the resource itself, never the host entity.
		UppercaseKind::Resource => {
			entity.world_scope(|world| -> Result<()> {
				write_resource_patch(
					world,
					app_registry,
					&named.name,
					patch.as_ref(),
				)
			})?;
		}
		UppercaseKind::Component => {
			insert_component(entity, patch.as_ref(), app_registry)?;
		}
	}
	Ok(())
}

/// Build a reflect patch for an uppercase tag's attributes against its type
/// info, so each value coerces to the field's concrete type.
pub(super) fn build_patch(
	el: &BsxElement,
	type_info: &'static bevy::reflect::TypeInfo,
	registry: &TypeRegistry,
	entity_refs: &HashMap<SmolStr, Entity>,
) -> Result<Box<dyn bevy::reflect::PartialReflect>> {
	use bevy::reflect::structs::DynamicStruct;
	let struct_info = match type_info {
		bevy::reflect::TypeInfo::Struct(info) => Some(info),
		_ => None,
	};
	let mut patch = DynamicStruct::default();
	for attr in &el.attributes {
		if is_directive(&attr.key) || attr.key.is_empty() {
			continue;
		}
		// an `@` binding becomes a field binding, not a patch field.
		if matches!(&attr.value, AttrValue::Expr(ValueExpr::Binding(_))) {
			continue;
		}
		let field_info = struct_info
			.and_then(|info| info.field(&attr.key))
			.and_then(|field| field.type_info());
		let literal = attr_to_literal(&attr.value)?;
		let mut resolver = entity_ref_resolver(entity_refs);
		// a `#[template]` stores an optional/required prop as `PropOpt<T>`; a markup
		// value must wrap into `PropOpt(Some(value))` to apply over the default.
		let reflected = match prop_opt_inner_info(field_info) {
			Some(inner_info) => prop_opt_value(
				&literal,
				field_info,
				inner_info,
				registry,
				&mut resolver,
			)?,
			None => DataLiteral::to_reflect(
				&literal,
				field_info,
				registry,
				&mut resolver,
			)?,
		};
		patch.insert_boxed(&attr.key, reflected);
	}
	// only a struct target can be represented by this `DynamicStruct`. A
	// tuple-struct/enum target (a tag-position literal like `<Name("x")/>` or
	// `<Log::Message("hi")/>`) builds from the literal instead, so this attribute
	// patch is unused; setting its represented type to a non-struct would panic.
	if struct_info.is_some() {
		patch.set_represented_type(Some(type_info));
	}
	Ok(Box::new(patch))
}

/// If `field_info` is a `PropOpt<T>` tuple struct, the inner `Option<T>`'s
/// [`TypeInfo`], else `None`. A `#[template]`'s optional/required props store as
/// `PropOpt<T>`, so a markup value targeting one must wrap into the option.
fn prop_opt_inner_info(
	field_info: Option<&'static bevy::reflect::TypeInfo>,
) -> Option<&'static bevy::reflect::TypeInfo> {
	let bevy::reflect::TypeInfo::TupleStruct(info) = field_info? else {
		return None;
	};
	if !info.type_path().contains("PropOpt<") {
		return None;
	}
	info.field_at(0).and_then(|field| field.type_info())
}

/// Build a `PropOpt(Some(value))` reflected value for a `PropOpt<T>` field, so a
/// markup prop value reaches a `#[template]`'s optional/required prop.
fn prop_opt_value(
	literal: &DataLiteral,
	field_info: Option<&'static bevy::reflect::TypeInfo>,
	option_info: &'static bevy::reflect::TypeInfo,
	registry: &TypeRegistry,
	resolver: &mut dyn FnMut(&str) -> Entity,
) -> Result<Box<dyn bevy::reflect::PartialReflect>> {
	use bevy::reflect::enums::DynamicEnum;
	use bevy::reflect::enums::DynamicVariant;
	use bevy::reflect::enums::VariantInfo;
	use bevy::reflect::tuple::DynamicTuple;
	use bevy::reflect::tuple_struct::DynamicTupleStruct;
	// the `Option<T>` carried by `PropOpt<T>(Option<T>)`; resolve the inner `T`.
	let inner_info = match option_info {
		bevy::reflect::TypeInfo::Enum(enum_info) => enum_info
			.variant("Some")
			.and_then(|variant| match variant {
				VariantInfo::Tuple(tuple) => tuple.field_at(0),
				_ => None,
			})
			.and_then(|field| field.type_info()),
		_ => None,
	};
	let inner =
		DataLiteral::to_reflect(literal, inner_info, registry, resolver)?;
	// `Some(inner)`
	let mut some = DynamicTuple::default();
	some.insert_boxed(inner);
	let mut option = DynamicEnum::new("Some", DynamicVariant::Tuple(some));
	option.set_represented_type(Some(option_info));
	// `PropOpt(Some(inner))`
	let mut prop_opt = DynamicTupleStruct::default();
	prop_opt.insert_boxed(Box::new(option));
	prop_opt.set_represented_type(field_info);
	Ok(Box::new(prop_opt))
}

/// Insert a reflect-patched component over its default onto `entity`.
pub(in crate::bsx) fn insert_component(
	entity: &mut EntityWorldMut,
	patch: &dyn bevy::reflect::PartialReflect,
	app_registry: &AppTypeRegistry,
) -> Result<()> {
	use bevy::ecs::reflect::ReflectComponent;
	let registry = app_registry.read();
	let type_info = patch.get_represented_type_info().ok_or_else(|| {
		bevyhow!("spread/component patch has no represented type")
	})?;
	let registration = registry.get(type_info.type_id()).ok_or_else(|| {
		bevyhow!("type `{}` is not registered", type_info.type_path())
	})?;
	let reflect_component =
		registration.data::<ReflectComponent>().ok_or_else(|| {
			bevyhow!(
				"type `{}` is not a registered component",
				type_info.type_path()
			)
		})?;
	// a `#[reflect(@RequiredField)]` field must be supplied by the patch, since the
	// `from_reflect`-over-default fill would otherwise mask a missing one.
	validate_required_fields(patch, type_info)?;
	// `from_reflect` the partial patch over default, then insert. A `DynamicStruct`
	// carrying only the provided fields fills the rest from the type's default.
	reflect_component.insert(entity, patch, &registry);
	Ok(())
}

/// Verify the patch supplies every `#[reflect(@RequiredField)]` field of its
/// represented type, erroring with the offending type and field name. A
/// `from_reflect`-over-default insert silently fills missing fields, so a
/// required field is enforced here instead (the markup twin of a `#[template]`'s
/// missing-required-prop check).
fn validate_required_fields(
	patch: &dyn bevy::reflect::PartialReflect,
	type_info: &bevy::reflect::TypeInfo,
) -> Result<()> {
	use bevy::reflect::ReflectRef;
	use bevy::reflect::TypeInfo;
	let type_name = type_info.type_path_table().short_path();
	match (type_info, patch.reflect_ref()) {
		(TypeInfo::Struct(info), ReflectRef::Struct(patch)) => {
			for field in info.iter() {
				if field.has_attribute::<RequiredField>()
					&& patch.field(field.name()).is_none()
				{
					bevybail!(
						"{type_name}: missing required field '{}'",
						field.name()
					);
				}
			}
		}
		(TypeInfo::TupleStruct(info), ReflectRef::TupleStruct(patch)) => {
			for index in 0..info.field_len() {
				let required = info.field_at(index).is_some_and(|field| {
					field.has_attribute::<RequiredField>()
				});
				if required && patch.field(index).is_none() {
					bevybail!("{type_name}: missing required field '{index}'");
				}
			}
		}
		_ => {}
	}
	Ok(())
}

// --- value/attribute lowering helpers ----------------------------------------
