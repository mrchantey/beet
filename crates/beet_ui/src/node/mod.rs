//! Re-exports of the `rsx!` / `#[template]` snippet runtime (now in
//! [`beet_core`](beet_core::types::snippet)) plus the small ui-side authoring
//! helpers ([`attr`]/[`optional_attr`]/[`test_world`]) that reference rendering
//! types.
//!
//! The snippet runtime ([`Snippet`](beet_core::prelude::Snippet),
//! [`IntoSnippet`](beet_core::prelude::IntoSnippet), … ) moved to
//! `beet_core::types::snippet`; this module re-exports it so the `rsx!`
//! lowering and `use beet_ui::prelude::*` call sites resolve unchanged.
mod node_ext;
pub use node_ext::*;

// re-export the moved snippet runtime so `beet_ui::prelude::*` keeps resolving
// `snippet`/`IntoSnippet`/`IntoSnippetBundle`/`PropOpt`/`BuildTemplate`/… for
// the macro output and existing call sites.
pub use beet_core::types::snippet::*;
