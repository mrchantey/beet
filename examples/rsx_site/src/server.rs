use crate::prelude::*;
use beet::prelude::*;

/// The beet brand green, `rgb(0, 255, 191)`, seeding the Material palette.
/// Private: every accent derives from it through the style system, so the theme
/// stays the single source of colour. The typed twin of a markup `<Theme>`.
const THEME_COLOR: Color = Color::srgb(0., 1., 0.75);

/// The site's render substrate, shared by the web and terminal entry points.
///
/// Adds the router observers and charcell pipeline (via [`RouterPlugin`]) plus
/// the Material style rule set that both the web [`Stylesheet`] and the charcell
/// renderer read, then registers the site-local [`design_row_rule`].
pub fn server_plugin(app: &mut App) {
	app.add_plugins((
		MinimalPlugins,
		// `RouterPlugin` brings in `ServerPlugin`, which installs the `HttpServer`
		// backend and registers the server types.
		RouterPlugin,
		LogPlugin::new(Level::INFO),
		// seeds the `Theme` resource with the brand colour; `rebuild_theme_tones`
		// derives every palette tone from it, the Rust analogue of a markup
		// `<Theme color=.../>` declaration.
		material::MaterialStylePlugin::new(THEME_COLOR),
	));
	// the site-local typed `Rule` (see `design_row_rule`), used by the buttons
	// showcase. The hero uses `inline_class!` instead, since its layout is a
	// single-use one-off.
	app.world_mut()
		.get_resource_or_init::<RuleSet>()
		.insert_rule(design_row_rule());
}

/// The site router: the typed page collection wrapped in the global
/// [`BeetLayout`] via the [`BaseLayout`] render middleware, so every route's body
/// is placed into the layout's `<main>`. The batteries-included [`default_router`]
/// adds `/app-info` and the cached `/js/reactivity.js` runtime the reactive
/// counter loads.
pub fn rsx_site_router() -> impl Bundle {
	(
		default_router(),
		pages_routes(),
		BaseLayout::<BeetLayout>::default(),
	)
}
