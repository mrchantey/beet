use crate::prelude::*;
use beet_core::prelude::*;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;

pub fn analytics_plugin(app: &mut App) {
	app.add_systems(PostStartup, spawn_analytics_event_store)
		.add_observer(handle_analytics_events);
}


#[derive(Clone, Deref, DerefMut, Resource)]
pub struct AnalyticsEventStore {
	pub store: TableStore<AnalyticsEvent>,
}
/// Spawn the analytics event store resource, using the
fn spawn_analytics_event_store(
	mut commands: Commands,
	ws_config: When<Res<WorkspaceConfig>>,
	pkg_config: When<Res<PackageConfig>>,
) {
	let fs_dir = ws_config.analytics_dir.into_abs();
	let bucket_name = pkg_config.analytics_bucket_name();
	let access = pkg_config.service_access;
	commands.run_system_cached_with(
		AsyncTask::spawn_with_queue,
		async move |queue| {
			let store = dynamo_fs_selector(&fs_dir, &bucket_name, access).await;
			queue.insert_resource(AnalyticsEventStore { store });
		},
	);
}


/// A listener for [`AnalyticsEvent`] triggers, pushing them to the [`AnalyticsEventStore`] resource
fn handle_analytics_events(
	trigger: On<AnalyticsEvent>,
	store: ResMut<AnalyticsEventStore>,
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

/// An event to be recorded, usually representing a user interaction on the site.
///
#[derive(Debug, Clone, Serialize, Deserialize, Event)]
pub struct AnalyticsEvent {
	pub id: Uuid,
	/// The performance.now() timestamp from the client when the event was recorded
	pub client_timestamp: u64,
	pub event_type: String,
	#[serde(flatten)]
	pub event_data: Value,
	#[serde(flatten)]
	pub session_data: Value,
}


impl AnalyticsEvent {
	pub fn parse(payload: Value) -> Result<Self> {
		// these fields are serde flattened, ensure they wont overwrite primary key
		if !payload["event_data"]["id"].is_null() {
			bevybail!("event_data.id field is not allowed");
		}
		if !payload["session_data"]["id"].is_null() {
			bevybail!("session_data.id field is not allowed");
		}

		let event_obj = payload["event_data"].as_object();
		let session_obj = payload["session_data"].as_object();
		if let (Some(event_map), Some(session_map)) = (event_obj, session_obj) {
			for key in event_map.keys() {
				if session_map.contains_key(key) {
					bevybail!(
						"conflicting field '{}' in event_data and session_data",
						key
					);
				}
			}
		}

		Self {
			id: Uuid::now_v7(),
			client_timestamp: payload["client_timestamp"].as_u64().unwrap_or(0),
			event_type: payload["event_type"]
				.as_str()
				.unwrap_or("unknown")
				.to_string(),
			event_data: payload["event_data"].clone(),
			session_data: payload["session_data"].clone(),
		}
		.xok()
	}
}

impl TableRow for AnalyticsEvent {
	fn id(&self) -> Uuid { self.id }
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use serde_json::Value;
	use serde_json::json;
	use sweet::prelude::*;

	fn event() -> Value {
		json! ({
			"client_timestamp": 123456,
			"event_type": "client_error",
			"event_data": {
				"message": "bar"
			},
			"session_data": {
				"session_id": "abc123"
			}
		})
	}


	#[test]
	fn not_allowed() {
		AnalyticsEvent::parse(json!({
			"event_data": { "id": "foo" }
		}))
		.xpect_err();
		AnalyticsEvent::parse(json!({
			"session_data": { "id": "foo" }
		}))
		.xpect_err();
		AnalyticsEvent::parse(json!({
			"event_data": { "foo": "bar" },
			"session_data": { "foo": "bar" }
		}))
		.xpect_err();
	}
	#[test]
	fn works() {
		let ev = AnalyticsEvent::parse(event()).unwrap();
		let json = serde_json::to_value(&ev).unwrap();
		json["id"].as_str().unwrap().len().xpect_eq(36);
		json["event_type"]
			.as_str()
			.unwrap()
			.xpect_eq("client_error");
		json["message"].as_str().unwrap().xpect_eq("bar");
	}
}
