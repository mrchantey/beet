//! The in-world navigator page-view analytics: recording a terminal page view
//! when a session navigates away from a page or closes.
use super::router_analytics_ext;
use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// Record page-view analytics for an in-world navigation to `url`: finalize the
/// previous page's dwell (emitting its page-view event) and start tracking the
/// new one. Called by the [`Navigator`] after it binds a page.
///
/// A no-op for network browsing (only the app's own in-world routes are its page
/// views) and without an analytics observer listening.
pub async fn record_page_view(entity: &AsyncEntity, url: &Url) -> Result {
	let now = Instant::now();
	let url = url.clone();
	// atomically roll the tracked page to the new one, taking the previous;
	// skipped for the network transport (not our own routes).
	let Some((session, previous)) = entity
		.get_mut(move |mut nav: Mut<Navigator>| {
			if !matches!(nav.transport(), NavigatorTransport::InWorld { .. }) {
				return None;
			}
			let previous = nav.take_current_page();
			nav.set_current_page(Some((now, url)));
			Some((nav.analytics_session(), previous))
		})
		.await?
	else {
		return Ok(());
	};
	if let Some((landed, previous_url)) = previous {
		let event = router_analytics_ext::page_view_event(
			session,
			&previous_url,
			now.duration_since(landed),
		);
		entity
			.world()
			.with(move |world: &mut World| world.trigger(event))
			.await;
	}
	Ok(())
}

/// Observer: when a [`Navigator`] is removed (a terminal session closing),
/// finalize its current page's dwell so the last page view is recorded.
///
/// Runs before the component is gone, so it reads the tracked page. Registered by
/// [`NavigatorPlugin`]; a no-op without an analytics observer listening.
pub fn finalize_page_view_on_remove(
	ev: On<Remove, Navigator>,
	navigators: Query<&Navigator>,
	mut commands: Commands,
) {
	if let Ok(nav) = navigators.get(ev.entity) {
		if let Some((landed, url)) = nav.current_page() {
			commands.trigger(router_analytics_ext::page_view_event(
				nav.analytics_session(),
				url,
				landed.elapsed(),
			));
		}
	}
}
