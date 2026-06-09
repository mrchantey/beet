use beet_core::prelude::*;

/// beet_ui's extension of the core [`DocumentPlugin`].
///
/// The document machinery itself lives in `beet_core`; this adds the beet_ui
/// side that cannot, namely the `action`-gated [`common_actions`](super) type
/// registrations and the `net`-gated [`refresh_blob_store_list`] system, which
/// depend on `beet_action` / `beet_net` (both downstream of `beet_core`).
#[derive(Default)]
pub struct DocumentUiPlugin;

impl Plugin for DocumentUiPlugin {
	fn build(&self, app: &mut App) {
		app.init_plugin::<DocumentPlugin>();

		// Register action types when the action feature is enabled
		#[cfg(feature = "action")]
		app.register_type::<super::Increment>()
			.register_type::<super::Decrement>()
			.register_type::<super::AddField>()
			.register_type::<super::SetField>()
			.register_type::<super::RemoveAtField>()
			.register_type::<super::ReadField>();

		// re-list a changed BlobStore into its backing field, before the sync
		// chain renders it, also after the async sync point
		#[cfg(feature = "net")]
		app.add_systems(
			PreUpdate,
			super::blob_store_list::refresh_blob_store_list
				.after(async_world_sync_point::<BeetAsyncSyncPoint>)
				.before(sync_document_to_local),
		);
	}
}
