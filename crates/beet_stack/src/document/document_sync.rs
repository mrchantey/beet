use crate::prelude::*;
use beet_core::prelude::*;



/// Tracks every [`FieldRef`] associated with this [`Document`] entity.
/// This entity may or may not have been initialized with a [`Document`]
#[derive(Component)]
#[relationship_target(relationship = FieldOf)]
pub struct Fields(Vec<Entity>);

/// Attached to a [`FieldRef`] to track its associated [`Document`].
/// As [`FieldRef`] is immutable, this is only added on insert and never
/// checked.
#[derive(Component)]
#[relationship(relationship_target = Fields)]
pub struct FieldOf {
	#[relationship]
	pub document: Entity,
}

pub(super) fn link_field_to_document(
	ev: On<Insert, FieldRef>,
	mut commands: Commands,
	fields: Query<&FieldRef>,
	mut doc_query: DocumentQuery,
) -> Result {
	let field = fields.get(ev.entity)?;
	let document = doc_query.entity(ev.entity, &field.document);
	commands.entity(ev.entity).insert(FieldOf { document });
	Ok(())
}
pub(super) fn unlink_field_from_document(
	ev: On<Remove, FieldRef>,
	mut commands: Commands,
) -> Result {
	commands.entity(ev.entity).remove::<FieldOf>();
	Ok(())
}


pub(super) fn update_text_fields(
	query: Populated<(&Document, &Fields), Changed<Document>>,
	mut text_fields: Query<(&FieldRef, &mut TextContent)>,
) -> Result {
	for (doc, doc_fields) in query {
		for field in doc_fields.iter() {
			if let Ok((field_ref, mut text)) = text_fields.get_mut(field) {
				let field = doc.get_field_ref(&field_ref.field_path)?;
				text.0 = field.to_string();
			}
		}
	}
	Ok(())
}
