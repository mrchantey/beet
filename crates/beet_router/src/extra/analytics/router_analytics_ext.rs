//! Free helpers shared across the router-side analytics emitters.
use beet_core::prelude::*;
use beet_net::prelude::*;

/// Build a terminal-session page-view [`AnalyticsEvent`] for `url` with the given
/// dwell duration.
pub fn page_view_event(
	session: Uuid,
	url: &Url,
	dwell: Duration,
) -> AnalyticsEvent {
	AnalyticsEvent::new(url.path_string(), AnalyticsEventData::PageView {
		duration_ms: dwell.as_millis() as u64,
		referrer: None,
		title: None,
		client: ClientDescriptor::default(),
	})
	.with_client_kind(ClientKind::Terminal)
	.with_session(Some(session))
}

/// The client kind for a routed request: a web client when it carries a user
/// agent, else a local CLI invocation (terminal sessions do not route through the
/// request middleware, so they are not a case here).
pub fn request_client_kind(has_user_agent: bool) -> ClientKind {
	if has_user_agent {
		ClientKind::Web
	} else {
		ClientKind::Cli
	}
}
