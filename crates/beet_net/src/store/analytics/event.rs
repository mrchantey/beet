//! The [`AnalyticsEvent`] wire type: common columns plus the typed,
//! variant-specific [`AnalyticsEventData`].
use super::analytics_ext;
use crate::prelude::*;
use beet_core::prelude::*;
use serde::Deserialize;
use serde::Serialize;

/// The kind of an [`AnalyticsEvent`], the discriminant of [`AnalyticsEventData`].
///
/// Stored as its own field so it is a cheap filter column (and, in a future SQL
/// table, a real column) rather than something a query digs out of the JSON data.
#[derive(
	Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
pub enum AnalyticsEventKind {
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

/// The variant-specific payload of an [`AnalyticsEvent`], stored as JSON.
///
/// The columns of [`AnalyticsEvent`] are what every event shares (who, where,
/// when); this enum is what differs per kind (a request's status, a page view's
/// dwell, a click's element). [`Self::kind`] is the discriminant that
/// [`AnalyticsEvent::event_kind`] mirrors.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AnalyticsEventData {
	/// A routed server request.
	Request {
		/// The response status code.
		status: u16,
		/// The request method.
		method: SmolStr,
		/// The `User-Agent` header, if any.
		user_agent: Option<SmolStr>,
	},
	/// A viewed page and its dwell duration.
	PageView {
		/// Dwell duration in milliseconds (the total, after heartbeat upserts).
		duration_ms: u64,
		/// The referring url, if any.
		referrer: Option<SmolStr>,
		/// The document title, if any.
		title: Option<SmolStr>,
		/// The browser client descriptor (user agent, screen, viewport, ...).
		client: ClientDescriptor,
	},
	/// A click on an interactive element.
	Click {
		/// Why it was recorded, eg `anchor-click` / `button-click`.
		reason: SmolStr,
		/// The clicked element.
		element: ClickElement,
	},
	/// A max scroll depth reached on a page.
	Scroll {
		/// The furthest scroll depth reached, as a percentage.
		max_percent: u32,
	},
	/// A client-side error.
	Error {
		/// Where it came from, eg `window` / `console`.
		source: SmolStr,
		/// The error message.
		message: SmolStr,
		/// The originating file, if any.
		file: Option<SmolStr>,
		/// The line number, if any.
		line: Option<u32>,
		/// The column number, if any.
		column: Option<u32>,
	},
}

impl AnalyticsEventData {
	/// The [`AnalyticsEventKind`] discriminant for this data.
	pub fn kind(&self) -> AnalyticsEventKind {
		match self {
			Self::Request { .. } => AnalyticsEventKind::Request,
			Self::PageView { .. } => AnalyticsEventKind::PageView,
			Self::Click { .. } => AnalyticsEventKind::Click,
			Self::Scroll { .. } => AnalyticsEventKind::Scroll,
			Self::Error { .. } => AnalyticsEventKind::Error,
		}
	}
}

/// The browser client descriptor a web page view carries: what kind of client is
/// viewing, no identifiers. Every field is optional (a partial beacon, or a
/// non-browser client, may omit any).
#[derive(
	Debug, Default, Clone, PartialEq, Serialize, Deserialize,
)]
pub struct ClientDescriptor {
	/// The `navigator.userAgent`.
	pub user_agent: Option<SmolStr>,
	/// The `navigator.language`.
	pub language: Option<SmolStr>,
	/// The `navigator.platform`.
	pub platform: Option<SmolStr>,
	/// The screen width in pixels.
	pub screen_width: Option<u32>,
	/// The screen height in pixels.
	pub screen_height: Option<u32>,
	/// The viewport width in pixels.
	pub viewport_width: Option<u32>,
	/// The viewport height in pixels.
	pub viewport_height: Option<u32>,
	/// The device pixel ratio.
	pub device_pixel_ratio: Option<f64>,
	/// The IANA timezone name.
	pub timezone: Option<SmolStr>,
}

/// The element a click event recorded, for locating what was clicked.
#[derive(
	Debug, Default, Clone, PartialEq, Serialize, Deserialize,
)]
pub struct ClickElement {
	/// The element tag name, eg `A` / `BUTTON`.
	pub tag: Option<SmolStr>,
	/// The element id attribute.
	pub id: Option<SmolStr>,
	/// The element class attribute.
	pub class: Option<SmolStr>,
	/// The element's text content (truncated by the client).
	pub text: Option<SmolStr>,
}

/// The coarse kind of client that produced an [`AnalyticsEvent`].
#[derive(
	Debug, Default, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize,
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

/// A single analytics record: common columns plus a typed [`AnalyticsEventData`].
///
/// The columns are what every event shares; [`Self::data`] is the per-kind
/// payload (stored as JSON), and [`Self::event_kind`] is its discriminant as a
/// filterable column. For a [`AnalyticsEventKind::PageView`] the primary key
/// [`Self::id`] is the client-generated page-view id, so re-recording it (a
/// heartbeat or the final `beforeunload`) overwrites the row in place.
#[derive(Debug, Clone, Serialize, Deserialize, Event)]
pub struct AnalyticsEvent {
	/// UUIDv7 primary key (time-sortable), doubling as the storage row id.
	pub id: Uuid,
	/// Server wall-clock time the event was recorded, ms since the unix epoch.
	pub timestamp: u64,
	/// The kind discriminant (mirrors [`Self::data`]'s variant), a filter column.
	pub event_kind: AnalyticsEventKind,
	/// The coarse client kind.
	pub client_kind: ClientKind,
	/// The session this event belongs to, if known. A terminal session is its
	/// [`Navigator`](../../../../beet_router); a web session is the client id.
	pub session: Option<Uuid>,
	/// The requested or viewed path, eg `/docs/intro`.
	pub path: SmolStr,
	/// ISO 3166-1 alpha-2 country code, derived from the client ip then
	/// discarded. Applies to any event whose client address is known.
	pub country: Option<SmolStr>,
	/// Raw client ip, only populated when [`AnalyticsConfig::store_ip`] is set;
	/// off by default so the default posture collects no personal data.
	pub ip: Option<SmolStr>,
	/// The variant-specific payload.
	pub data: AnalyticsEventData,
}

impl AnalyticsEvent {
	/// A fresh event for `path` carrying `data`, stamped with a new UUIDv7 and the
	/// current server time. [`Self::event_kind`] is taken from `data`.
	pub fn new(path: impl Into<SmolStr>, data: AnalyticsEventData) -> Self {
		Self {
			id: uuid_ext::now_v7(),
			timestamp: analytics_ext::now_ms(),
			event_kind: data.kind(),
			client_kind: ClientKind::Unknown,
			session: None,
			path: path.into(),
			country: None,
			ip: None,
			data,
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

	/// Parses a web client beacon body into an event.
	///
	/// The client sends `{kind, page_view_id, session, path, ...}` where the
	/// remaining fields depend on the kind; they build the typed
	/// [`AnalyticsEventData`]. `session_cookie` (from the request cookie) and the
	/// geoip `country` are supplied by the server; the body's `session` wins when
	/// present. A page view keeps the client `page_view_id` as its row id (so
	/// heartbeats overwrite); other kinds get a fresh id.
	pub fn from_beacon(
		body: Value,
		session_cookie: Option<Uuid>,
		ip: Option<SmolStr>,
		country: Option<SmolStr>,
	) -> Result<Self> {
		// a non-object body (eg an unparsed `text/plain` beacon) would default
		// every field into a phantom `/` page view, so reject it instead.
		if !matches!(body, Value::Map(_)) {
			bevybail!("beacon body must be a json object, got: {body:?}");
		}
		let str = |key: &str| {
			body.get(key).and_then(|value| value.as_str().ok()).map(SmolStr::from)
		};
		let u64 = |key: &str| body.get(key).and_then(|value| value.as_u64().ok());
		let uuid = |key: &str| {
			body.get(key)
				.and_then(|value| value.as_str().ok())
				.and_then(|str| str.parse::<Uuid>().ok())
		};

		// the kind selects the typed payload; the client never sends `request`, so
		// an unknown/absent kind falls back to a page view. A nested object (client /
		// element) deserializes into its typed struct, defaulting when malformed.
		let data = match body.get("kind").and_then(|value| value.as_str().ok()) {
			Some("click") => AnalyticsEventData::Click {
				reason: str("reason").unwrap_or_default(),
				element: body
					.get("element")
					.cloned()
					.and_then(|value| value.into_serde().ok())
					.unwrap_or_default(),
			},
			Some("scroll") => AnalyticsEventData::Scroll {
				max_percent: u64("max_percent").unwrap_or(0) as u32,
			},
			Some("error") => AnalyticsEventData::Error {
				source: str("source").unwrap_or_default(),
				message: str("message").unwrap_or_default(),
				file: str("file"),
				line: u64("line").map(|line| line as u32),
				column: u64("column").map(|column| column as u32),
			},
			_ => AnalyticsEventData::PageView {
				duration_ms: u64("duration_ms").unwrap_or(0),
				referrer: str("referrer"),
				title: str("title"),
				client: body
					.get("client")
					.cloned()
					.and_then(|value| value.into_serde().ok())
					.unwrap_or_default(),
			},
		};

		let path = str("path").unwrap_or_else(|| "/".into());
		let mut event = Self::new(path, data)
			.with_client_kind(ClientKind::Web)
			.with_session(uuid("session").or(session_cookie));
		// a page view keeps the client id so heartbeats overwrite; others are new.
		if let AnalyticsEventKind::PageView = event.event_kind {
			if let Some(id) = uuid("page_view_id") {
				event.id = id;
			}
		}
		event.country = country;
		event.ip = ip;
		event.xok()
	}
}

/// The storage row impl (needs the json store surface).
#[cfg(feature = "json")]
impl TableStoreRow for AnalyticsEvent {
	fn id(&self) -> Uuid { self.id }
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	fn page_view_body() -> Value {
		val!({
			"kind": "page_view",
			"page_view_id": "0192f8a0-0000-7000-8000-000000000000",
			"session": "0192f8a0-0000-7000-8000-0000000000ff",
			"path": "/docs/intro",
			"duration_ms": 4200u64,
			"referrer": "https://beet.dev",
			"client": { "user_agent": "Mozilla/5.0" }
		})
	}

	#[beet_core::test]
	fn parses_page_view_beacon() {
		let event =
			AnalyticsEvent::from_beacon(page_view_body(), None, None, None)
				.unwrap();
		event.event_kind.xpect_eq(AnalyticsEventKind::PageView);
		event.client_kind.xpect_eq(ClientKind::Web);
		event.path.as_str().xpect_eq("/docs/intro");
		event
			.id
			.to_string()
			.xpect_eq("0192f8a0-0000-7000-8000-000000000000");
		match event.data {
			AnalyticsEventData::PageView {
				duration_ms,
				client,
				..
			} => {
				duration_ms.xpect_eq(4200);
				// the nested client descriptor deserialized into its typed struct.
				client.user_agent.as_deref().xpect_eq(Some("Mozilla/5.0"));
			}
			_ => panic!("expected a page view"),
		}
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
		click.event_kind.xpect_eq(AnalyticsEventKind::Click);
		matches!(
			click.data,
			AnalyticsEventData::Click { .. }
		)
		.xpect_true();

		AnalyticsEvent::from_beacon(
			val!({ "kind": "scroll", "path": "/", "max_percent": 80u64 }),
			None,
			None,
			None,
		)
		.unwrap()
		.event_kind
		.xpect_eq(AnalyticsEventKind::Scroll);

		AnalyticsEvent::from_beacon(
			val!({ "kind": "error", "path": "/", "message": "boom" }),
			None,
			None,
			None,
		)
		.unwrap()
		.event_kind
		.xpect_eq(AnalyticsEventKind::Error);
	}

	/// A non-object body (eg an unparsed `text/plain` beacon) is rejected rather
	/// than defaulting into a phantom `/` page view.
	#[beet_core::test]
	fn rejects_non_object_body() {
		AnalyticsEvent::from_beacon(
			Value::str(r#"{"path":"/docs"}"#),
			None,
			None,
			None,
		)
		.xpect_err();
	}

	#[beet_core::test]
	fn body_session_wins_over_cookie() {
		let cookie = uuid_ext::now_v7();
		let event =
			AnalyticsEvent::from_beacon(page_view_body(), Some(cookie), None, None)
				.unwrap();
		event
			.session
			.unwrap()
			.to_string()
			.xpect_eq("0192f8a0-0000-7000-8000-0000000000ff");
	}

	#[beet_core::test]
	fn cookie_session_fills_when_body_absent() {
		let cookie = uuid_ext::now_v7();
		let body = val!({ "path": "/", "page_view_id": "not-a-uuid" });
		let event =
			AnalyticsEvent::from_beacon(body, Some(cookie), None, None).unwrap();
		event.session.xpect_eq(Some(cookie));
		event.id.xpect_not_eq(Uuid::nil());
	}
}
