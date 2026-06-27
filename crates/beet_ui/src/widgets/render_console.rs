//! `RenderConsole` widget — a scrollable on-page console mirroring
//! `console.log/info/warn/error` into a styled panel.
//!
//! Web target only. Dropped into a page (eg beside a `<Wasm>` loader), it captures
//! a headless wasm program's output: beet's `log` macros route to `console.*` on
//! wasm, so `info!`/`warn!`/`error!` surface here for free. Colors lean on the
//! material `:root` tokens the page's `<Stylesheet>` emits, with hard fallbacks so
//! it reads even without one.
use beet_core::prelude::*;

/// Emits the `#beet-console` panel with its bundled `<style>` and the `<script>`
/// that wraps `console.*` to append a styled line per call (see
/// `render_console.js`). The panel is the single root; the inline `<script>` runs
/// once its ancestor element is parsed, so the wrap captures every later log.
#[template]
pub fn RenderConsole() -> impl Bundle {
	rsx! {
		<div id="beet-console">
			<style>{include_str!("./render_console.css")}</style>
			<script>{include_str!("./render_console.js")}</script>
		</div>
	}
}
