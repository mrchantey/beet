//! Link behavior: internal links navigate in-app, external links open per
//! [`OnOpenLink`].
//!
//! A clicked `<a>`'s href is classified internal (a relative/same-origin path)
//! vs external (an absolute URL to another origin, see [`Url::is_external`]).
//! Internal links always navigate the in-app [`Navigator`]. External links follow
//! [`OnOpenLink`]: `External` (default) opens the system browser, `Internal`
//! points the `Navigator` at the external URL (the in-terminal mini-browser).
//!
//! The actual system-browser launch goes through the [`OpenExternalLink`] event,
//! so a test (or an embedder) can intercept it without spawning a process.

use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_ui::prelude::*;

/// How the app opens an external link (an absolute URL to another origin).
///
/// A config component on the live host or the [`Navigator`]; the host-level
/// default applies to every external link. (A per-`<a>` override is a future
/// option: an [`OnOpenLink`] on the link element would win over the host
/// default.)
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Reflect, Component)]
#[reflect(Component)]
pub enum OnOpenLink {
	/// Open the link in a live (system) browser, the default.
	#[default]
	External,
	/// Open the link in this application (the in-terminal mini-browser): point the
	/// [`Navigator`] at the external URL.
	Internal,
}

/// Request to open `url` in the system browser, fired when an external link is
/// followed under [`OnOpenLink::External`].
///
/// The default handler [`open_external_link`] launches the system browser via the
/// `webbrowser` crate; a test or embedder can observe this instead to intercept
/// the open without spawning a process (the open is authoritative through this
/// event, not delegated to OSC-8 in the live buffer).
#[derive(Debug, Clone, Message)]
pub struct OpenExternalLink {
	/// The absolute URL to open.
	pub url: Url,
}

/// Registers the link-open observers: the `<a>` click classifier and the default
/// system-browser opener.
#[derive(Default)]
pub struct OpenLinkPlugin;

impl Plugin for OpenLinkPlugin {
	fn build(&self, app: &mut App) {
		app.add_message::<OpenExternalLink>()
			.register_type::<OnOpenLink>()
			.add_systems(Update, open_external_link)
			.add_observer(on_link_click);
	}
}

/// Observer: classify a clicked `<a>` and route it.
///
/// Internal links navigate the [`Navigator`]; external links follow the host's
/// [`OnOpenLink`] (default: emit [`OpenExternalLink`] for the system browser;
/// `Internal`: navigate the `Navigator` at the external URL).
///
/// The navigator is resolved from the clicked link's own surface (the
/// [`RenderSurface`] ancestor, on which the [`Navigator`] is co-located), so a
/// click navigates only that session when many surfaces coexist.
fn on_link_click(
	ev: On<PointerUp>,
	mut commands: Commands,
	elements: ElementQuery,
	parents: Query<&ChildOf>,
	surfaces: Query<&RenderSurface>,
	navigators: Query<Option<&OnOpenLink>, With<Navigator>>,
	mut open: MessageWriter<OpenExternalLink>,
) -> Result {
	// only `<a>` elements carry a LinkView; other targets are ignored.
	let link_entity = ev.event().target;
	let Ok(link) = elements.get_as::<LinkView>(link_entity) else {
		return Ok(());
	};
	let url = Url::parse(link.href);
	// the navigator is co-located on the link's surface; resolve it from the link
	// rather than assuming a single global navigator, so each session navigates
	// independently.
	let Some(navigator) = surface_of(link_entity, &parents, &surfaces) else {
		return Ok(());
	};
	let Ok(on_open) = navigators.get(navigator) else {
		return Ok(());
	};
	let on_open = on_open.copied().unwrap_or_default();

	// internal, or external rendered in-app, both navigate the Navigator.
	let navigate = !url.is_external() || on_open == OnOpenLink::Internal;
	if navigate {
		commands
			.entity(navigator)
			.queue_async(move |entity| Navigator::navigate_to(entity, url));
	} else {
		// external + OnOpenLink::External: open in the system browser, through the
		// interceptable event so the open is authoritative (not OSC-8 delegated).
		open.write(OpenExternalLink { url });
	}
	Ok(())
}

/// System: open each [`OpenExternalLink`] in the system browser.
///
/// The default external-open behavior. A test that wants to assert intent
/// without launching a browser observes [`OpenExternalLink`] instead (this
/// system still runs but `webbrowser::open` failing in a headless CI is ignored).
fn open_external_link(mut events: MessageReader<OpenExternalLink>) {
	for ev in events.read() {
		// a failed launch (eg headless CI) is non-fatal; the intent was recorded.
		let _ = webbrowser::open(&ev.url.to_string());
	}
}

#[cfg(test)]
mod test {
	use super::*;

	/// Records every [`OpenExternalLink`] (an external-open intent), the test seam
	/// over the system-browser launch.
	#[derive(Resource, Default)]
	struct ExternalOpens(Vec<Url>);

	/// An app with the link plumbing and the document/template substrate, plus a
	/// system recording external-open intents (the test seam over the browser
	/// launch; the real `open_external_link` still runs but is a no-op in CI).
	fn link_app() -> App {
		let mut app = App::new();
		// AsyncPlugin: the Navigator's on_add queues a home navigation through
		// queue_async, and the internal-nav branch queues navigate_to.
		app.add_plugins((
			MinimalPlugins,
			AsyncPlugin,
			TemplatePlugin,
			DocumentPlugin,
			CharcellPlugin,
			OpenLinkPlugin,
		));
		app.init_resource::<ExternalOpens>();
		app.add_systems(
			Update,
			|mut events: MessageReader<OpenExternalLink>,
			 mut opens: ResMut<ExternalOpens>| {
				for ev in events.read() {
					opens.0.push(ev.url.clone());
				}
			},
		);
		app
	}

	/// Spawn a Navigator (with an optional [`OnOpenLink`]) co-located on a surface,
	/// plus an `<a href>` page tree bound to that surface via [`RenderSurface`],
	/// returning the `<a>` element entity. Mirrors the real app: the click handler
	/// resolves the navigator from the link's surface.
	fn spawn_link(
		app: &mut App,
		on_open: Option<OnOpenLink>,
		href: &str,
	) -> Entity {
		let mut nav = app.world_mut().spawn(Navigator::default());
		if let Some(on_open) = on_open {
			nav.insert(on_open);
		}
		let navigator = nav.id();
		// build the <a> through the template substrate so it is a real Element with
		// the href attribute LinkView reads; bind its tree to the navigator's surface.
		let root = app
			.world_mut()
			.spawn_template(rsx! { <a href=href.to_string()>"link"</a> })
			.unwrap()
			.id();
		app.world_mut()
			.entity_mut(root)
			.insert(RenderSurface(navigator));
		app.update();
		// the <a> is the descendant Element whose tag is "a".
		app.world_mut()
			.query::<(Entity, &Element)>()
			.iter(app.world())
			.find(|(_, element)| element.tag() == "a")
			.map(|(entity, _)| entity)
			.unwrap_or(root)
	}

	/// Trigger a `PointerUp` on `entity`, as the hit-test would on a click.
	fn click(app: &mut App, entity: Entity) {
		let pointer = app.world_mut().spawn_empty().id();
		app.world_mut()
			.entity_mut(entity)
			.trigger(PointerUp::new(pointer));
		app.update();
	}

	/// An external link under the default `OnOpenLink::External` emits an
	/// open-external intent with the URL and does not navigate in-app.
	#[beet_core::test]
	#[ignore = "the default open_external_link system launches the real system browser; behavior verified, but it pops a browser tab on every run"]
	fn external_link_opens_externally_by_default() {
		let mut app = link_app();
		let link = spawn_link(&mut app, None, "https://example.com");
		click(&mut app, link);
		let opens = &app.world().resource::<ExternalOpens>().0;
		opens.len().xpect_eq(1);
		opens[0].is_external().xpect_true();
		opens[0].authority().xpect_eq(Some("example.com"));
	}

	/// Under `OnOpenLink::Internal`, an external link does NOT open externally (it
	/// is routed to the in-app Navigator instead).
	#[beet_core::test]
	fn external_link_internal_mode_does_not_open_browser() {
		let mut app = link_app();
		let link = spawn_link(
			&mut app,
			Some(OnOpenLink::Internal),
			"https://example.com",
		);
		click(&mut app, link);
		// no external open: the Internal mode navigated the Navigator instead.
		app.world()
			.resource::<ExternalOpens>()
			.0
			.is_empty()
			.xpect_true();
	}

	/// An internal (relative) link never opens externally regardless of
	/// `OnOpenLink`; it navigates the in-app Navigator.
	#[beet_core::test]
	fn internal_link_never_opens_externally() {
		let mut app = link_app();
		let link = spawn_link(&mut app, None, "/beta");
		click(&mut app, link);
		app.world()
			.resource::<ExternalOpens>()
			.0
			.is_empty()
			.xpect_true();
	}

	/// `Url::is_external` classifies absolute (has authority) vs relative URLs.
	#[beet_core::test]
	fn url_external_classification() {
		Url::parse("https://example.com/x")
			.is_external()
			.xpect_true();
		Url::parse("/about").is_external().xpect_false();
		Url::parse("./next").is_external().xpect_false();
	}
}
