//! Running a request: the `Router` action, the plugin that builds its routes,
//! and the exchange front-ends a route is authored with.

mod exchange_overload;
mod exchange_sequence;
// the `<FieldRoute>` document-field route front-end. std-only: it names the
// `beet_ui` document actions, which ride `dep:beet_ui`.
#[cfg(feature = "std")]
mod field_route;
// the `ExchangeScript` route marker, the `<ScriptRoute>` front-end, and the
// `ExchangeScriptElement` console-capturing `<script>` entry action.
#[cfg(feature = "scripting")]
mod exchange_script;
// The `Router` dispatch action and the route-building `RouterPlugin` are shared
// across std and no_std (one `Router` type, one plugin). The single builder that
// assembles them with the standard middleware and app routes is `Router::with_defaults`
// (in `extra`). The std-only scene/help rendering pipeline stays feature-gated
// inside these and in the `help`/`sidebar` modules under `site`; the no_std build
// falls back to a plain-text route listing.
mod route_matrix;
mod router;
mod router_plugin;
mod server_action_client;

pub use exchange_overload::*;
#[cfg(feature = "scripting")]
pub use exchange_script::*;
pub use exchange_sequence::*;
#[cfg(feature = "std")]
pub use field_route::*;
pub use route_matrix::*;
pub use router::*;
pub use router_plugin::*;
pub use server_action_client::*;
