use crate::prelude::*;
use beet_core::prelude::AsyncTask;
use bevy::prelude::*;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;


#[derive(Clone, Deref, DerefMut, Resource)]
pub struct AnalyticsEventStore<T: TableContent = BasicEventPayload> {
	pub store: TableStore<AnalyticsEvent<T>>,
}


/// A listener for [`AnalyticsEvent`] triggers, pushing them to the [`AnalyticsEventStore`] resource
pub fn handle_analytics_events<T: TableContent>(
	trigger: Trigger<AnalyticsEvent<T>>,
	store: ResMut<AnalyticsEventStore<T>>,
	mut commands: Commands,
) {
	let store = store.clone();
	let event = trigger.event().clone();
	commands.run_system_cached_with(
		AsyncTask::spawn_with_queue_unwrap,
		async move |_| {
			store.push(event).await?;
			Ok(())
		},
	);
}

impl<T: TableContent> TableData for AnalyticsEvent<T> {
	fn id(&self) -> Uuid { self.id }
}

/// An event to be recorded, usually representing a user interaction on the site
#[derive(Debug, Clone, Serialize, Deserialize, Event)]
pub struct AnalyticsEvent<T = BasicEventPayload> {
	pub id: Uuid,
	/// The path where the event took place
	pub path: RoutePath,
	/// The payload for the event, usually a superset of [`BasicEventPayload`]
	pub payload: T,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BasicEventPayload {
	Visit,
	Click { element_id: String },
	ClientError { id: String, message: String },
	Other(Value),
}

impl AnalyticsEvent {
	pub fn new(path: impl Into<RoutePath>, payload: BasicEventPayload) -> Self {
		Self {
			id: Uuid::now_v7(),
			path: path.into(),
			payload,
		}
	}
}
