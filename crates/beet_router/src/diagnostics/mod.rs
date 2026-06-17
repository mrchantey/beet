//! The render diagnostics layer: a validation pass that recovers the
//! "type-checked feeling" for a no-code site.
//!
//! [`render_diagnostics`] walks a built render tree against the route set and
//! [`RuleSet`](beet_ui::prelude::RuleSet), surfacing unknown tags, broken
//! internal links and unknown classes; [`check_routes`] drives it across every
//! static route for `beet check`, `export-static` and the dev serve. The
//! [`RenderDiagnostics`] resource configures each check's severity.

mod check_routes;
// the machine-readable export of the site's tags/classes/routes/style-props for a
// future editor; needs `serde_json` to serialize and the BSX registries (`bsx`,
// pulled in by `std`) to enumerate handler/template tags.
#[cfg(all(feature = "json", feature = "bsx"))]
mod manifest;
mod render_diagnostics;

pub use check_routes::*;
#[cfg(all(feature = "json", feature = "bsx"))]
pub use manifest::*;
pub use render_diagnostics::*;
