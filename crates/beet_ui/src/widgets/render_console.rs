//! `RenderConsole` widget — a scrollable on-page console mirroring
//! `console.log/info/warn/error` into a styled panel.
//!
//! Web target only. Dropped into a page (eg beside a `<Wasm>` loader), it captures
//! a headless wasm program's output: beet's `log` macros route to `console.*` on
//! wasm, so `info!`/`warn!`/`error!` surface here for free. The panel and per-line
//! level colors are styled by the [`console`](crate::style::material::classes::console)
//! rules, emitted into the page by the `<Stylesheet>` in its `<head>`.
use crate::prelude::*;
use beet_core::prelude::*;

/// Emits the `#beet-console` panel and the `<script>` that wraps `console.*` to
/// append a styled line per call (see `render_console.js`). The panel is the
/// single root; the inline `<script>` runs once its ancestor element is parsed,
/// so the wrap captures every later log.
///
/// The `id` is the script's handle (`getElementById("beet-console")`) and the
/// `beet-console-panel` class carries its styling; the per-line classes the
/// script sets (`beet-console-error`/`warn`/`info`) are styled by the same rule
/// module.
#[template]
pub fn RenderConsole() -> impl Bundle {
	rsx! {
		<div id="beet-console" {Classes::new([classes::CONSOLE_PANEL])}>
			<script>{include_str!("./render_console.js")}</script>
		</div>
	}
}
