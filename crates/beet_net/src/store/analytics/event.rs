//! The [`AnalyticsEvent`] wire type and its [`AnalyticsKind`] / [`ClientKind`]
//! discriminants.
use super::analytics_ext;
use crate::prelude::*;
use beet_core::prelude::*;
use serde::Deserialize;
use serde::Serialize;

/// The kind of an [`AnalyticsEvent`].
///
/// [`Self::Request`] is the server traffic log; the rest are client-reported: a
/// [`Self::PageView`] with a dwell duration, plus the lighter interaction and
/// error events a page emits.
#[derive(
	Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect,
)]
pub enum AnalyticsKind {
	/// A routed server request: the raw traffic log, one per request.
	Request,
	/// A viewed page with a dwell duration: web beacon or in-world navigator.
	PageView,
	/// A click on an interactive element (a button or link).
	Click,
	/// A max scroll depth reached on a page.
	Scroll,
	/// A client-side error (a window error event or a `console.error`).
	Error,
}

impl AnalyticsKind {
	/// The kind named by a beacon body's `kind` field, defaulting to
	/// [`Self::PageView`] when absent or unrecognized.
	pub fn from_beacon(value: Option<&Value>) -> Self {
		match value.and_then(|value| value.as_str().ok()) {
			Some("click") => Self::Click,
			Some("scroll") => Self::Scroll,
			Some("error") => Self::Error,
			_ => Self::PageView,
		}
	}

	/// Whether this kind is a page view (the only kind that carries a dwell
	/// duration and an upsert row id).
	pub fn is_page_view(&self) -> bool { matches!(self, Self::PageView) }
}

/// The coarse kind of client that produced an [`AnalyticsEvent`].
///
/// The bucket for grouping; finer detail (user agent, terminal name, window
/// size) lives in [`AnalyticsEvent::data`].
#[derive(
	Debug,
	Default,
	Clone,
	Copy,
	PartialEq,
	Eq,
	Hash,
	Serialize,
	Deserialize,
	Reflect,
)]
pub enum ClientKind {
	/// A web browser over HTTP.
	Web,
	/// A terminal client over SSH or the local TUI.
	Terminal,
	/// A local command-line invocation.
	Cli,
	/// An unrecognized client.
	#[default]
	Unknown,
}

/// A single analytics record: a request, a viewed page, or a client interaction.
///
/// For a [`AnalyticsKind::PageView`] the primary key [`Self::id`] is the
/// client-generated page-view id, so re-recording it (a heartbeat or the final
/// `beforeunload`) overwrites the row in place, leaving [`Self::duration_ms`] as
/// the total dwell rather than a value reconstructed at query time.
#[derive(Debug, Clone, Serialize, Deserialize, Event)]
pub struct AnalyticsEvent {
	/// UUIDv7 primary key (time-sortable), doubling as the storage row id.
	pub id: Uuid,
	/// Server wall-clock time the event was recorded, ms since the unix epoch.
	pub timestamp: u64,
	/// Whether this is a request, page-view, or interaction record.
	pub kind: AnalyticsKind,
	/// The coarse client kind.
	pub client_kind: ClientKind,
	/// The session this event belongs to, if known. A terminal session is its
	/// [`Navigator`](../../../../beet_router); a web session is the client id.
	pub session: Option<Uuid>,
	/// The requested or viewed path, eg `/docs/intro`.
	pub path: SmolStr,
	/// Response status for a [`AnalyticsKind::Request`].
	pub status: Option<u16>,
	/// Dwell duration in milliseconds for a [`AnalyticsKind::PageView`].
	pub duration_ms: Option<u64>,
	/// ISO 3166-1 alpha-2 country code, derived from the client ip then
	/// discarded. `None` when geoip is unavailable or the ip is unresolvable.
	pub country: Option<SmolStr>,
	/// Raw client ip, only populated when [`AnalyticsConfig::store_ip`] is set;
	/// off by default so the default posture collects no personal data.
	pub ip: Option<SmolStr>,
	/// Client descriptor and event-specific extras (user agent, terminal name,
	/// window size, referrer, clicked element, scroll depth, error message, ...),
	/// a flat [`Value::Map`].
	pub data: Value,
}

impl AnalyticsEvent {
	/// A fresh event of `kind` for `path`, stamped with a new UUIDv7 and the
	/// current server time. Other fields start empty for the caller to fill.
	pub fn new(kind: AnalyticsKind, path: impl Into<SmolStr>) -> Self {
		Self {
			id: Uuid::now_v7(),
			timestamp: analytics_ext::now_ms(),
			kind,
			client_kind: ClientKind::Unknown,
			session: None,
			path: path.into(),
			status: None,
			duration_ms: None,
			country: None,
			ip: None,
			data: Value::map(),
		}
	}

	/// A [`AnalyticsKind::PageView`] with an explicit `id`, so a heartbeat or
	/// final beacon re-recording the same id overwrites the row in place.
	pub fn page_view(id: Uuid, path: impl Into<SmolStr>) -> Self {
		Self {
			id,
			..Self::new(AnalyticsKind::PageView, path)
		}
	}

	/// Builder-style setter for the client kind.
	pub fn with_client_kind(mut self, client_kind: ClientKind) -> Self {
		self.client_kind = client_kind;
		self
	}

	/// Builder-style setter for the session id.
	pub fn with_session(mut self, session: Option<Uuid>) -> Self {
		self.session = session;
		self
	}

	/// Builder-style setter for the dwell duration.
	pub fn with_duration_ms(mut self, duration_ms: u64) -> Self {
		self.duration_ms = Some(duration_ms);
		self
	}

	/// Inserts a key into the [`Self::data`] descriptor map, ignoring non-map
	/// data (which [`Self::new`] never produces).
	pub fn insert_data(
		&mut self,
		key: impl Into<SmolStr>,
		value: impl Into<Value>,
	) {
		if let Ok(map) = self.data.as_map_mut() {
			map.insert(key, value);
		}
	}

	/// Body keys read as typed fields rather than folded into [`Self::data`].
	const RESERVED_KEYS: [&'static str; 5] =
		["kind", "page_view_id", "session", "path", "duration_ms"];

	/// Parses a web client beacon body into an event.
	///
	/// The client sends `{kind, page_view_id, session, path, duration_ms, ...}`;
	/// any remaining fields (the client descriptor, a clicked element, a scroll
	/// depth, an error message) fold into [`Self::data`]. `session_cookie` (from
	/// the request cookie) and the geoip `country` are supplied by the server; the
	/// body's `session` wins when present. A page view keeps the client-generated
	/// `page_view_id` as its row id (so heartbeats overwrite); other kinds get a
	/// fresh id.
	pub fn from_beacon(
		body: Value,
		session_cookie: Option<Uuid>,
		ip: Option<SmolStr>,
		country: Option<SmolStr>,
	) -> Result<Self> {
		let parse_uuid = |value: Option<&Value>| {
			value
				.and_then(|value| value.as_str().ok())
				.and_then(|str| str.parse::<Uuid>().ok())
		};
		let kind = AnalyticsKind::from_beacon(body.get("kind"));
		let path = body
			.get("path")
			.and_then(|value| value.as_str().ok())
			.unwrap_or("/")
			.to_string();

		let mut event = Self::new(kind, path);
		// a page view keeps the client id so heartbeats overwrite; others are new.
		if kind.is_page_view() {
			if let Some(id) = parse_uuid(body.get("page_view_id")) {
				event.id = id;
			}
			event.duration_ms =
				body.get("duration_ms").and_then(|value| value.as_u64().ok());
		}
		event.client_kind = ClientKind::Web;
		event.session = parse_uuid(body.get("session")).or(session_cookie);
		event.country = country;
		event.ip = ip;
		// fold every non-reserved field into the descriptor map.
		if let Ok(map) = body.as_map() {
			for (key, value) in map {
				if !Self::RESERVED_KEYS.contains(&key.as_str()) {
					event.insert_data(key.clone(), value.clone());
				}
			}
		}
		event.xok()
	}
}

impl TableStoreRow for AnalyticsEvent {
	fn id(&self) -> Uuid { self.id }
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	fn beacon_body() -> Value {
		val!({
			"page_view_id": "0192f8a0-0000-7000-8000-000000000000",
			"session": "0192f8a0-0000-7000-8000-0000000000ff",
			"path": "/docs/intro",
			"duration_ms": 4200u64,
			"referrer": "https://beet.dev",
			"client": {
				"user_agent": "Mozilla/5.0",
				"screen_width": 1920u64
			}
		})
	}

	#[beet_core::test]
	fn parses_page_view_beacon() {
		let event = AnalyticsEvent::from_beacon(beacon_body(), None, None, None)
			.unwrap();
		event.kind.xpect_eq(AnalyticsKind::PageView);
		event.client_kind.xpect_eq(ClientKind::Web);
		event.path.as_str().xpect_eq("/docs/intro");
		event.duration_ms.xpect_eq(Some(4200));
		event
			.id
			.to_string()
			.xpect_eq("0192f8a0-0000-7000-8000-000000000000");
		event
			.data
			.get("client")
			.unwrap()
			.get("user_agent")
			.unwrap()
			.as_str()
			.unwrap()
			.xpect_eq("Mozilla/5.0");
	}

	#[beet_core::test]
	fn parses_interaction_kinds_with_fresh_ids() {
		let click = AnalyticsEvent::from_beacon(
			val!({
				"kind": "click",
				"path": "/",
				"reason": "button-click",
				"element": { "tag": "BUTTON" }
			}),
			None,
			None,
			None,
		)
		.unwrap();
		click.kind.xpect_eq(AnalyticsKind::Click);
		// a non-page-view gets a fresh id, never the page-view upsert key
		click.duration_ms.xpect_none();
		click
			.data
			.get("reason")
			.unwrap()
			.as_str()
			.unwrap()
			.xpect_eq("button-click");

		AnalyticsEvent::from_beacon(
			val!({ "kind": "scroll", "path": "/", "max_percent": 80u64 }),
			None,
			None,
			None,
		)
		.unwrap()
		.kind
		.xpect_eq(AnalyticsKind::Scroll);

		AnalyticsEvent::from_beacon(
			val!({ "kind": "error", "path": "/", "message": "boom" }),
			None,
			None,
			None,
		)
		.unwrap()
		.kind
		.xpect_eq(AnalyticsKind::Error);
	}

	#[beet_core::test]
	fn body_session_wins_over_cookie() {
		let cookie = Uuid::now_v7();
		let event =
			AnalyticsEvent::from_beacon(beacon_body(), Some(cookie), None, None)
				.unwrap();
		event
			.session
			.unwrap()
			.to_string()
			.xpect_eq("0192f8a0-0000-7000-8000-0000000000ff");
	}

	#[beet_core::test]
	fn cookie_session_fills_when_body_absent() {
		let cookie = Uuid::now_v7();
		let body = val!({ "path": "/", "page_view_id": "not-a-uuid" });
		let event =
			AnalyticsEvent::from_beacon(body, Some(cookie), None, None).unwrap();
		event.session.xpect_eq(Some(cookie));
		event.id.xpect_not_eq(Uuid::nil());
	}
}
