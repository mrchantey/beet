//! `@`-binding lowering: a binding expression to the sync components of its
//! source, against whichever entity the selector targets.

use super::entity_refs::*;
use super::literal::*;
use crate::prelude::*;
use bevy::ecs::template::TemplateContext;

/// Lower a text/attribute-position value expression onto `entity`: a literal
/// becomes a [`Value`], an `@` binding its binding components
/// ([`apply_binding`], with `comp_target` naming the entity an `@comp` binds),
/// a `$`reference is rejected (an entity reference is not a text value).
pub(super) fn apply_value_expr(
	expr: &ValueExpr,
	entity: &mut EntityWorldMut,
	comp_target: BindingTarget,
) -> Result<()> {
	match expr {
		ValueExpr::Literal(literal) => {
			entity.insert(literal_to_value(literal)?);
		}
		ValueExpr::Binding(binding) => {
			apply_binding(binding, entity, comp_target)?;
		}
		ValueExpr::EntityRef(_) => {
			bevybail!(
				"`$name` entity references are not valid in text position"
			)
		}
	}
	Ok(())
}

/// Lower an `@` binding's components onto the value-bearing `entity`.
///
/// `comp_target` is the entity an `@comp` binding's component lives on: the
/// element in attribute position, the binding entity itself in text and spread
/// position, the `$ref` entity when a selector is present.
pub(super) fn apply_binding(
	binding: &BindingExpr,
	entity: &mut EntityWorldMut,
	comp_target: BindingTarget,
) -> Result<()> {
	match binding.source {
		BindingSource::Doc => {
			entity
				.insert(field_ref(&binding.field_path, binding.init.as_ref())?);
		}
		BindingSource::Prop => {
			entity.insert(
				FieldRef::new(binding.field_path.clone())
					.with_document(DocumentPath::Props),
			);
		}
		// the `Value`<->reflect bridge needs `serde_json` (the `json` feature);
		// an embedded build without it keeps the `Value` but loses the
		// resource/component sync (Risk: documented, acceptable).
		BindingSource::Res => {
			insert_value_if_missing(entity);
			#[cfg(feature = "json")]
			entity.insert(ResourceFieldRef::new(
				binding.type_path.clone().unwrap_or_default(),
				binding.field_path.to_string(),
			));
		}
		BindingSource::Comp => {
			insert_value_if_missing(entity);
			#[cfg(feature = "json")]
			{
				let mut reflect_ref = ReflectFieldRef::new(
					binding.type_path.clone().unwrap_or_default(),
					binding.field_path.to_string(),
				);
				reflect_ref.target = comp_target;
				entity.insert(reflect_ref);
			}
			#[cfg(not(feature = "json"))]
			let _ = comp_target;
		}
	}
	Ok(())
}

/// Seed a default [`Value`] for a binding's sync to fill, preserving any value
/// already present (eg a `FieldRef`-seeded one).
fn insert_value_if_missing(entity: &mut EntityWorldMut) {
	if !entity.contains::<Value>() {
		entity.insert(Value::default());
	}
}

/// The target of an `@entity:name::` selector in a cx-bearing position (text,
/// event): a reserved name resolves to its well-known entity (or defers to the
/// sync pass), else through the `bx:ref` machinery.
pub(super) fn selector_target(
	name: &SmolStr,
	refs: &RefBindings,
	cx: &mut TemplateContext,
) -> BindingTarget {
	match ReservedRef::parse(name) {
		Some(reserved) => reserved.target(name, cx),
		None => BindingTarget::Entity(resolve_ref(name, refs, cx)),
	}
}

/// The target of an `@entity:name::` selector resolved against a pre-built
/// name->entity map ([`resolve_entity_refs`], which also resolves the
/// build-time reserved names): a lazy reserved name defers to the sync pass,
/// anything else looks up the map.
pub(super) fn map_selector_target(
	name: &SmolStr,
	entity_refs: &HashMap<SmolStr, Entity>,
) -> BindingTarget {
	match ReservedRef::parse(name) {
		Some(ReservedRef::PageRoot | ReservedRef::Router) => {
			BindingTarget::Reserved(name.clone())
		}
		_ => BindingTarget::Entity(
			entity_refs
				.get(name)
				.copied()
				.unwrap_or(Entity::PLACEHOLDER),
		),
	}
}

/// The `@comp` target of an attribute-position expression: the `$ref` entity
/// when selected, else the `element` carrying the attribute.
pub(super) fn attr_comp_target(
	expr: &ValueExpr,
	element: Entity,
	entity_refs: &HashMap<SmolStr, Entity>,
) -> BindingTarget {
	match expr {
		ValueExpr::Binding(BindingExpr {
			selector: Some(name),
			..
		}) => map_selector_target(name, entity_refs),
		_ => BindingTarget::Entity(element),
	}
}

/// Build a [`FieldRef`] from a `@doc:field=init` binding.
fn field_ref(path: &FieldPath, init: Option<&DataLiteral>) -> Result<FieldRef> {
	let mut field = FieldRef::new(path.clone());
	if let Some(init) = init {
		field = field.with_init(literal_to_value(init)?);
	}
	Ok(field)
}
