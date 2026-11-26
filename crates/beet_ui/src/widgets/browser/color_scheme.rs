//! `ColorSchemeScript` widget: an inline `<script>` applying the visitor's
//! stored colour preferences before first paint, and [`ColorSchemeToggle`],
//! the button cycling the stored scheme.
//!
//! Web target only: non-web targets carry the scheme class directly on an
//! ancestor element. Drop it into the document `<head>` (eg the `head` slot) so
//! the scheme is applied before the body paints, avoiding a flash of the wrong
//! theme.
use crate::prelude::*;
use crate::style::material::classes;
use beet_core::prelude::*;

/// Emits the bundled colour preferences script as an inline `<script>`.
///
/// The preferences are a [`SceneBody`] in `localStorage`, laid out as a
/// `LocalStorageStore` named `preferences` lays out `scene.json`, beside a
/// `tones.css` of palette overrides derived from it, so the browser applies
/// them with no colour maths and a wasm runtime reads the same document as an
/// ordinary store file. The scheme is the stored [`Theme`]'s
/// [`scheme`](Theme::scheme) (`{"beet_ui::Theme": {"scheme": "Dark"}}`),
/// over the OS `prefers-color-scheme` and under a `?color-scheme=light|dark`
/// pin, toggling the matching class on `<html>`.
///
/// `globalThis.beetPreferences` is the document's api for a page's scripts,
/// `setColorScheme("light" | "dark" | null)` the scheme's, and a change in any
/// tab dispatches `beet-preferences` on `document`; the script's header
/// comment lists the api. A pin also points the browser chrome at that
/// scheme's [`Head`](crate::prelude::Head) tint, so the script belongs after
/// `<Head>`'s metas, in its slot, where every layout puts it.
#[template]
pub fn ColorSchemeScript() -> impl Bundle {
	// inject the shared names so the script, the style rules and the scene
	// format stay in lockstep.
	let body = format!(
		"const LIGHT_SCHEME={:?},DARK_SCHEME={:?},THEME={:?};\n{}",
		classes::LIGHT_SCHEME.as_selector(),
		classes::DARK_SCHEME.as_selector(),
		Theme::type_path(),
		include_str!("./color_scheme.js"),
	);
	rsx! {
		<script>{body}</script>
	}
}

/// A header button cycling the stored colour scheme through auto, light and
/// dark, its icon naming the current one: a half filled circle, a sun or a
/// moon, line drawn in the text colour rather than glyphs a font may lack.
/// The click is bound by [`ColorSchemeScript`], which also swaps the icon, so
/// it renders the auto icon and does nothing without the script, ie on the
/// terminal, where it is hidden.
#[template]
pub fn ColorSchemeToggle() -> impl Bundle {
	rsx! {
		<button
			type="button"
			data-color-scheme-toggle="auto"
			aria-label="Toggle colour scheme"
			title="Colour scheme: auto"
			{Classes::new([classes::BTN, classes::BTN_TEXT, classes::TERMINAL_HIDDEN])}>
			<svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true">
				<circle cx="12" cy="12" r="9"/>
				<path d="M12 3a9 9 0 0 0 0 18z" fill="currentColor"/>
			</svg>
		</button>
	}
}

#[cfg(all(test, feature = "scene"))]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_scene::prelude::TemplateLoader;

	/// The document the script and the site's editors write is an ordinary
	/// scene: its theme loads onto the resource, its keys hex strings as
	/// `<Theme>` attributes are, and an entity of a type this world never
	/// registered is held rather than refused.
	#[beet_core::test]
	fn preferences_load_as_a_scene() {
		// a runtime's world registers `Name`, never the site's own `Logo`
		let mut world = world_ext::ui_app()
			.add_plugins(material::MaterialStylePlugin)
			.register_type::<Name>()
			.world_mut()
			.xmap(core::mem::take);
		let body = MediaType::Json
			.deserialize::<SceneBody>(
				br##"{
				"$type": "org.beet.scene#body",
				"resources": {
					"beet_ui::Theme": { "scheme": "Light", "primary": "#ff0000" }
				},
				"entities": {
					"0": {
						"bevy_ecs::name::Name": "chunky",
						"beet_site::Logo": { "gap": 0.3 }
					}
				}
			}"##,
			)
			.unwrap();
		let spawned = TemplateLoader::new(&mut world).load_body(&body).unwrap();
		let theme = world.resource::<Theme>();
		theme.scheme.xpect_eq(ColorScheme::Light);
		theme.primary.xpect_eq(Some(Color::srgb_u8(255, 0, 0)));
		let favourite = world.entity(spawned[0]);
		favourite.get::<Name>().unwrap().as_str().xpect_eq("chunky");
		favourite
			.get::<Unregistered>()
			.unwrap()
			.contains("beet_site::Logo")
			.xpect_true();
	}
}
