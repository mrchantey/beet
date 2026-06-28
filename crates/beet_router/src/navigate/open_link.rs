//! Link behavior: internal links navigate in-app, external links open per
//! [`OnOpenLink`].
//!
//! A clicked `<a>`'s href is classified internal (a relative/same-origin path)
//! vs external (an absolute URL to another origin, see [`Url::is_external`]).
//! Internal links always navigate the in-app [`Navigator`]. External links follow
//! [`OnOpenLink`]: `External` (default) hands the link off outside the app,
//! `Internal` points the `Navigator` at the external URL (the in-terminal
//! mini-browser).
//!
//! How an external hand-off happens depends on the surface's terminal, not a
//! config flag: a remote (SSH) surface (a
//! [`ChannelTerminal`](beet_ui::prelude::ChannelTerminal)) can't reach the user's
//! browser, so the URL is copied to the *client's* clipboard via
//! [`CopyToClipboard`](beet_ui::prelude::CopyToClipboard) (which works over SSH);
//! a local surface launches the system browser through the [`OpenExternalLink`]
//! event, which a test (or embedder) can intercept without spawning a process.

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
/// followed under [`OnOpenLink::External`] on a *local* surface.
///
/// The default handler [`open_external_link`] launches the system browser via the
/// `webbrowser` crate (native) or opens a new tab via `window.open` (wasm); a test
/// or embedder can observe this instead to intercept the open without spawning a
/// process. Not fired on a remote (SSH) surface, which copies the URL to the
/// client clipboard via [`CopyToClipboard`](beet_ui::prelude::CopyToClipboard)
/// instead.
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
/// [`OnOpenLink`] (`Internal`: navigate the `Navigator` at the external URL;
/// `External`, the default: hand off outside the app). The hand-off is chosen by
/// the surface's terminal: a remote (SSH) surface (a
/// [`ChannelTerminal`](beet_ui::prelude::ChannelTerminal)) can't reach the user's
/// browser, so the URL is copied to the *client's* clipboard via
/// [`CopyToClipboard`](beet_ui::prelude::CopyToClipboard); a local surface opens
/// the system browser via [`OpenExternalLink`].
///
/// The navigator is resolved from the clicked link's own surface (the
/// [`RenderSurface`] ancestor, on which the [`Navigator`] is co-located), so a
/// click acts only on that session when many surfaces coexist.
fn on_link_click(
	ev: On<PointerUp>,
	mut commands: Commands,
	elements: ElementQuery,
	surfaces: SurfaceQuery,
	navigators: Query<Option<&OnOpenLink>, With<Navigator>>,
	mut open: MessageWriter<OpenExternalLink>,
	// a remote surface routes external links to the client clipboard instead of a
	// server-side browser; both only exist under the terminal renderer.
	#[cfg(feature = "tui")] remote: Query<(), With<ChannelTerminal>>,
	#[cfg(feature = "tui")] mut copy: MessageWriter<CopyToClipboard>,
) -> Result {
	// only `<a>` elements carry a LinkView; other targets are ignored.
	let link_entity = ev.event().target;
	let Ok(link) = elements.get_as::<LinkView>(link_entity) else {
		return Ok(());
	};
	let url = Url::parse(link.href);
	// the navigator is co-located on the link's surface; resolve it from the link
	// rather than assuming a single global navigator, so each session acts
	// independently.
	let Some(navigator) = surfaces.surface_of(link_entity) else {
		return Ok(());
	};
	let Ok(on_open) = navigators.get(navigator) else {
		return Ok(());
	};
	let on_open = on_open.copied().unwrap_or_default();

	// internal, or external rendered in-app, both navigate the Navigator.
	if !url.is_external() || on_open == OnOpenLink::Internal {
		commands.entity(navigator).queue_async(move |entity| async move {
			// a session can close (despawning its co-located navigator) between the
			// click and this task, eg a multi-tenant SSH client that disconnects
			// mid-navigation. A despawned navigator is a clean no-op; a genuine load
			// failure is logged rather than escalated to the command error handler,
			// as the boot navigation in `Navigator::on_add` also does.
			if !entity.is_alive().await {
				return;
			}
			if let Err(err) = Navigator::navigate_to(entity, url).await {
				error!("navigation failed: {err}");
			}
		});
		return Ok(());
	}

	// external + OnOpenLink::External: hand off outside the app.
	#[cfg(feature = "tui")]
	if remote.contains(navigator) {
		// remote (SSH): copy to the client's clipboard (a server-side browser
		// would open on the wrong machine).
		copy.write(CopyToClipboard {
			surface: navigator,
			content: url.to_string().into(),
		});
		return Ok(());
	}
	// local: open in the system browser, through the interceptable event so the
	// open is authoritative (not OSC-8 delegated).
	open.write(OpenExternalLink { url });
	Ok(())
}

/// System: open each [`OpenExternalLink`] in the system browser (native) or a
/// new browser tab (wasm).
///
/// The default external-open behavior. A test that wants to assert intent
/// without launching a browser observes [`OpenExternalLink`] instead (this
/// system still runs but `webbrowser::open` failing in a headless CI is ignored).
fn open_external_link(mut events: MessageReader<OpenExternalLink>) {
	for ev in events.read() {
		let url = ev.url.to_string();
		// native: a failed launch (eg headless CI) is non-fatal; the intent was
		// recorded. `webbrowser` is native-only.
		#[cfg(not(target_arch = "wasm32"))]
		let _ = webbrowser::open(&url);
		// wasm: the browser opens the link in a new tab. A failure (no window, or a
		// popup blocker) is surfaced as an error rather than dropped silently.
		#[cfg(target_arch = "wasm32")]
		if let Err(err) = open_in_new_tab(&url) {
			error!("{err}");
		}
	}
}

/// Open `url` in a new browser tab via `window.open(url, "_blank")`.
///
/// Errors when there is no window, the call throws, or the browser returns no
/// window handle (a popup blocker), so the caller can report the failure.
#[cfg(target_arch = "wasm32")]
fn open_in_new_tab(url: &str) -> Result {
	use beet_core::exports::web_sys;
	let window = web_sys::window().ok_or_else(|| {
		bevyhow!("no browser window available to open `{url}`")
	})?;
	window
		.open_with_url_and_target(url, "_blank")
		.map_err(|err| bevyhow!("browser refused to open `{url}`: {err:?}"))?
		.ok_or_else(|| {
			bevyhow!(
				"browser blocked opening `{url}` in a new tab (popup blocker?)"
			)
		})?;
	Ok(())
}

#[cfg(test)]
mod test {
	use super::*;

	/// Records every [`OpenExternalLink`] (a local browser-open intent), the test
	/// seam over the system-browser launch.
	#[derive(Resource, Default)]
	struct ExternalOpens(Vec<Url>);

	/// Records every [`CopyToClipboard`] (a remote clipboard-copy intent), the test
	/// seam over the OSC-52 write.
	#[cfg(feature = "tui")]
	#[derive(Resource, Default)]
	struct ClipboardCopies(Vec<SmolStr>);

	/// An app with the link plumbing and the document/template substrate, plus
	/// systems recording the two external-link intents (the test seams over the
	/// browser launch and the OSC-52 write).
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
		#[cfg(feature = "tui")]
		{
			app.init_resource::<ClipboardCopies>();
			app.add_systems(
				Update,
				|mut events: MessageReader<CopyToClipboard>,
				 mut copies: ResMut<ClipboardCopies>| {
					for ev in events.read() {
						copies.0.push(ev.content.clone());
					}
				},
			);
		}
		app
	}

	/// Spawn a Navigator (with an optional [`OnOpenLink`]) co-located on a surface,
	/// plus an `<a href>` page tree bound to that surface via [`RenderSurface`],
	/// returning the `<a>` element entity. A `remote` surface also carries a
	/// [`ChannelTerminal`](beet_ui::prelude::ChannelTerminal), the SSH marker the
	/// handler keys off. Mirrors the real app: the click handler resolves the
	/// navigator from the link's surface.
	fn spawn_link(
		app: &mut App,
		on_open: Option<OnOpenLink>,
		remote: bool,
		href: &str,
	) -> Entity {
		let mut nav = app.world_mut().spawn(Navigator::default());
		if let Some(on_open) = on_open {
			nav.insert(on_open);
		}
		// a remote (SSH) surface is marked by a ChannelTerminal; only the terminal
		// renderer defines it, so the marker (and remote routing) is tui-only.
		#[cfg(feature = "tui")]
		if remote {
			nav.insert(ChannelTerminal::new(TerminalConfig::default()).0);
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

	/// A local surface opens an external link in the system browser via
	/// [`OpenExternalLink`].
	#[beet_core::test]
	#[ignore = "the open_external_link system launches the real system browser; behavior verified, but it pops a browser tab on every run"]
	fn local_external_link_opens_browser() {
		let mut app = link_app();
		let link = spawn_link(&mut app, None, false, "https://example.com");
		click(&mut app, link);
		let opens = &app.world().resource::<ExternalOpens>().0;
		opens.len().xpect_eq(1);
		opens[0].authority().xpect_eq(Some("example.com"));
	}

	/// A remote (SSH) surface copies an external link to the client clipboard
	/// instead of launching a server-side browser.
	#[cfg(feature = "tui")]
	#[beet_core::test]
	fn remote_external_link_copies_to_clipboard() {
		let mut app = link_app();
		let link = spawn_link(&mut app, None, true, "https://example.com");
		click(&mut app, link);
		// copied to the client, not opened server-side (the URL is canonicalized
		// with a trailing slash by `Url::to_string`).
		app.world()
			.resource::<ClipboardCopies>()
			.0
			.xpect_eq(vec![SmolStr::new("https://example.com/")]);
		app.world()
			.resource::<ExternalOpens>()
			.0
			.is_empty()
			.xpect_true();
	}

	/// Under `OnOpenLink::Internal`, an external link does NOT hand off (it is
	/// routed to the in-app Navigator instead), even on a remote surface.
	#[beet_core::test]
	fn external_link_internal_mode_navigates_in_app() {
		let mut app = link_app();
		let link = spawn_link(
			&mut app,
			Some(OnOpenLink::Internal),
			true,
			"https://example.com",
		);
		click(&mut app, link);
		// no hand-off: the Internal mode navigated the Navigator instead.
		app.world()
			.resource::<ExternalOpens>()
			.0
			.is_empty()
			.xpect_true();
		#[cfg(feature = "tui")]
		app.world()
			.resource::<ClipboardCopies>()
			.0
			.is_empty()
			.xpect_true();
	}

	/// An internal (relative) link never hands off regardless of surface; it
	/// navigates the in-app Navigator.
	#[beet_core::test]
	fn internal_link_never_hands_off() {
		let mut app = link_app();
		let link = spawn_link(&mut app, None, true, "/beta");
		click(&mut app, link);
		app.world()
			.resource::<ExternalOpens>()
			.0
			.is_empty()
			.xpect_true();
		#[cfg(feature = "tui")]
		app.world()
			.resource::<ClipboardCopies>()
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
