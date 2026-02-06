use crate::document::*;
use beet_core::prelude::*;


#[derive(Default)]
pub struct DocumentPlugin;



impl Plugin for DocumentPlugin {
	fn build(&self, app: &mut App) {
		app.add_observer(link_field_to_document)
			.add_observer(unlink_field_from_document)
			.add_systems(PreUpdate, update_text_fields);
	}
}
