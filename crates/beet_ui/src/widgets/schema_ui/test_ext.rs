//! The schema-driven widgets' own test helpers: building a generated form, and
//! locating the controls a schema emitted rather than an author wrote.
//!
//! Layered on the shared widget harness ([`test_ext`](crate::widgets::test_ext))
//! rather than restating it — which it re-exports, so a schema test names one
//! harness and every generic renderer, driver and world still has one home.
pub(super) use crate::widgets::test_ext::*;

use super::collection_edit::CollectionButton;
use super::collection_edit::CollectionEdit;
use super::variant_select::VariantSelect;
use crate::prelude::*;
use beet_core::prelude::*;

/// Render a [`DynamicForm`] for `schema`, bound to a `"field"` key, to HTML.
pub(super) fn form_html(schema: ValueSchema) -> String {
	render_html(rsx! {
		<DynamicForm schema={schema} field={FieldRef::new("field")}/>
	})
}

/// Build a form over `schema` bound to `field` of `document`, settled, returning
/// `(world, document root)`.
pub(super) fn build_form(
	schema: ValueSchema,
	field: &str,
	document: Value,
) -> (World, Entity) {
	let mut world = form_world();
	let field = FieldRef::new(field);
	let root = world
		.spawn_template(rsx! {
			<div>
				<DynamicForm schema={schema} field={field}/>
			</div>
		})
		.unwrap()
		.id();
	world.entity_mut(root).insert(Document::new(document));
	settle_world(&mut world);
	(world, root)
}

/// The value a document entity holds, ie what an edit through the generated
/// controls landed as.
pub(super) fn document_of(world: &mut World, entity: Entity) -> Value {
	world.entity(entity).get::<Document>().unwrap().0.clone()
}

/// The one button that submits the form it sits in, ie the only one no
/// `type="button"` excludes ([`Button`]'s `action`) — which is every generated
/// collection button.
pub(super) fn submit_button(world: &mut World) -> Entity {
	let actions = world
		.query_once::<(&Attribute, &Value, &AttributeOf)>()
		.into_iter()
		.filter(|(attribute, value, _)| {
			attribute.as_str() == "type"
				&& value
					.as_str()
					.map(|value| value == "button")
					.unwrap_or_default()
		})
		.map(|(_, _, attribute_of)| **attribute_of)
		.collect::<HashSet<_>>();
	elements_in(world, "button")
		.into_iter()
		.find(|button| !actions.contains(button))
		.expect("no submit button")
}

/// The generated control bound to `path`, ie the leaf a form emitted for it.
pub(super) fn bound(world: &mut World, path: &str) -> Entity {
	world
		.query_once::<(Entity, &ResolvedFieldPath)>()
		.into_iter()
		.find(|(_, resolved)| resolved.field_path.to_string() == path)
		.map(|(entity, _)| entity)
		.unwrap_or_else(|| panic!("no control is bound to `{path}`"))
}

/// The generated variant `<select>` choosing the enum at `path`, which binds no
/// field of its own (its value is the variant name).
pub(super) fn variant_select(world: &mut World, path: &str) -> Entity {
	world
		.query_once::<(Entity, &VariantSelect)>()
		.into_iter()
		.find(|(_, select)| select.field.field_path.to_string() == path)
		.map(|(entity, _)| entity)
		.unwrap_or_else(|| panic!("no variant select chooses `{path}`"))
}

/// The generated add button of the collection control bound to `path`.
pub(super) fn collection_add(world: &mut World, path: &str) -> Entity {
	world
		.query_once::<(Entity, &CollectionButton)>()
		.into_iter()
		.find(|(_, button)| {
			button.field.field_path.to_string() == path
				&& matches!(
					button.edit,
					CollectionEdit::Push(_) | CollectionEdit::Insert(_)
				)
		})
		.map(|(entity, _)| entity)
		.unwrap_or_else(|| panic!("no collection add button edits `{path}`"))
}
