use beet::prelude::*;

/// Showcases every Material color role as a swatch, in both the light and dark
/// schemes. Each swatch fills its background with the color-role token and its
/// text with the matching "on" token.
///
/// escape hatch: this page is inherently a web color-palette showcase. Token to
/// token references (background = `var(--material-colors-primary)`, foreground = `var(--material-colors-on-primary)`)
/// cannot be expressed through `Classes`/`inline_class!` in scene-mode `rsx!`
/// (which only lifts plain `Component` block attributes, not the `OnSpawn` that
/// `inline_class!` returns), so the swatch grid uses a raw `<style>` block keyed
/// off the design-token CSS variables the active rule set emits, eg
/// `--material-colors-primary`, `--material-colors-on-primary`. This is web
/// only; the terminal target skips `<style>`.
pub fn get() -> impl Scene {
	rsx! {
		<article>
			<h1>"Color Schemes"</h1>
			<h2>"Light"</h2>
			<div {Classes::new([classes::LIGHT_SCHEME])}>{scheme()}</div>
			<h2>"Dark"</h2>
			<div {Classes::new([classes::DARK_SCHEME])}>{scheme()}</div>
			<style>{SWATCH_CSS}</style>
		</article>
	}
}

/// One full set of color-role swatches, grouped by Material role family.
fn scheme() -> impl Scene {
	rsx! {
		<div {Classes::new(["scheme"])}>
			<h3>"Primary"</h3>
			<div {Classes::new(["color-group"])}>
				{swatch("Primary", "primary")}
				{swatch("On Primary", "on-primary")}
				{swatch("Primary Container", "primary-container")}
				{swatch("On Primary Container", "on-primary-container")}
				{swatch("Inverse Primary", "inverse-primary")}
			</div>
			<h3>"Secondary"</h3>
			<div {Classes::new(["color-group"])}>
				{swatch("Secondary", "secondary")}
				{swatch("On Secondary", "on-secondary")}
				{swatch("Secondary Container", "secondary-container")}
				{swatch("On Secondary Container", "on-secondary-container")}
			</div>
			<h3>"Tertiary"</h3>
			<div {Classes::new(["color-group"])}>
				{swatch("Tertiary", "tertiary")}
				{swatch("On Tertiary", "on-tertiary")}
				{swatch("Tertiary Container", "tertiary-container")}
				{swatch("On Tertiary Container", "on-tertiary-container")}
			</div>
			<h3>"Error"</h3>
			<div {Classes::new(["color-group"])}>
				{swatch("Error", "error")}
				{swatch("On Error", "on-error")}
				{swatch("Error Container", "error-container")}
				{swatch("On Error Container", "on-error-container")}
			</div>
			<h3>"Surface"</h3>
			<div {Classes::new(["color-group"])}>
				{swatch("Surface Dim", "surface-dim")}
				{swatch("Surface", "surface")}
				{swatch("Surface Bright", "surface-bright")}
				{swatch("On Surface", "on-surface")}
			</div>
			<div {Classes::new(["color-group"])}>
				{swatch("Surface Variant", "surface-variant")}
				{swatch("On Surface Variant", "on-surface-variant")}
				{swatch("Inverse Surface", "inverse-surface")}
				{swatch("Inverse On Surface", "inverse-on-surface")}
			</div>
			<div {Classes::new(["color-group"])}>
				{swatch("Surface Container Lowest", "surface-container-lowest")}
				{swatch("Surface Container Low", "surface-container-low")}
				{swatch("Surface Container", "surface-container")}
				{swatch("Surface Container High", "surface-container-high")}
				{swatch("Surface Container Highest", "surface-container-highest")}
			</div>
			<h3>"Misc"</h3>
			<div {Classes::new(["color-group"])}>
				{swatch("Outline", "outline")}
				{swatch("Outline Variant", "outline-variant")}
				{swatch("Scrim", "scrim")}
				{swatch("Shadow", "shadow")}
			</div>
		</div>
	}
}

/// A single labelled swatch carrying a role class for the `<style>` block.
fn swatch(label: &str, role: &str) -> impl Scene {
	let label = label.to_string();
	let classes =
		Classes::new([ClassName::string("color-box"), ClassName::string(role)]);
	rsx! {
		<div {classes}>{label}</div>
	}
}

/// Web-only swatch styling, keyed off the design-token CSS variables.
const SWATCH_CSS: &str = r#"
.color-group { display: flex; flex-direction: row; gap: 2px; flex-wrap: wrap; margin-bottom: 2px; }
.color-box { display: flex; align-items: center; padding: 0.3rem; height: 3rem; width: 10rem; }

.primary { background-color: var(--material-colors-primary); color: var(--material-colors-on-primary); }
.on-primary { background-color: var(--material-colors-on-primary); color: var(--material-colors-primary); }
.primary-container { background-color: var(--material-colors-primary-container); color: var(--material-colors-on-primary-container); }
.on-primary-container { background-color: var(--material-colors-on-primary-container); color: var(--material-colors-primary-container); }
.inverse-primary { background-color: var(--material-colors-inverse-primary); color: var(--material-colors-inverse-surface); }

.secondary { background-color: var(--material-colors-secondary); color: var(--material-colors-on-secondary); }
.on-secondary { background-color: var(--material-colors-on-secondary); color: var(--material-colors-secondary); }
.secondary-container { background-color: var(--material-colors-secondary-container); color: var(--material-colors-on-secondary-container); }
.on-secondary-container { background-color: var(--material-colors-on-secondary-container); color: var(--material-colors-secondary-container); }

.tertiary { background-color: var(--material-colors-tertiary); color: var(--material-colors-on-tertiary); }
.on-tertiary { background-color: var(--material-colors-on-tertiary); color: var(--material-colors-tertiary); }
.tertiary-container { background-color: var(--material-colors-tertiary-container); color: var(--material-colors-on-tertiary-container); }
.on-tertiary-container { background-color: var(--material-colors-on-tertiary-container); color: var(--material-colors-tertiary-container); }

.error { background-color: var(--material-colors-error); color: var(--material-colors-on-error); }
.on-error { background-color: var(--material-colors-on-error); color: var(--material-colors-error); }
.error-container { background-color: var(--material-colors-error-container); color: var(--material-colors-on-error-container); }
.on-error-container { background-color: var(--material-colors-on-error-container); color: var(--material-colors-error-container); }

.surface-dim { background-color: var(--material-colors-surface-dim); color: var(--material-colors-on-surface); }
.surface { background-color: var(--material-colors-surface); color: var(--material-colors-on-surface); }
.surface-bright { background-color: var(--material-colors-surface-bright); color: var(--material-colors-on-surface); }
.on-surface { background-color: var(--material-colors-on-surface); color: var(--material-colors-surface); }
.surface-variant { background-color: var(--material-colors-surface-variant); color: var(--material-colors-on-surface-variant); }
.on-surface-variant { background-color: var(--material-colors-on-surface-variant); color: var(--material-colors-surface-variant); }
.inverse-surface { background-color: var(--material-colors-inverse-surface); color: var(--material-colors-inverse-on-surface); }
.inverse-on-surface { background-color: var(--material-colors-inverse-on-surface); color: var(--material-colors-inverse-surface); }
.surface-container-lowest { background-color: var(--material-colors-surface-container-lowest); color: var(--material-colors-on-surface); }
.surface-container-low { background-color: var(--material-colors-surface-container-low); color: var(--material-colors-on-surface); }
.surface-container { background-color: var(--material-colors-surface-container); color: var(--material-colors-on-surface); }
.surface-container-high { background-color: var(--material-colors-surface-container-high); color: var(--material-colors-on-surface); }
.surface-container-highest { background-color: var(--material-colors-surface-container-highest); color: var(--material-colors-on-surface); }

.outline { background-color: var(--material-colors-outline); color: var(--material-colors-surface); }
.outline-variant { background-color: var(--material-colors-outline-variant); color: var(--material-colors-on-surface); }
.scrim { background-color: var(--material-colors-scrim); color: var(--material-colors-surface); }
.shadow { background-color: var(--material-colors-shadow); color: var(--material-colors-surface); }
"#;
