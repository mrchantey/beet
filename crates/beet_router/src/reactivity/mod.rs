//! The thin-client reactivity runtime widget ([`ReactivityScript`]) and its
//! JavaScript (`reactivity.js`).
//!
//! The browser half of the reactive wire format: the reactive
//! [`HtmlRenderer`](beet_ui::prelude::HtmlRenderer) emits `data-bx-*` annotations
//! and JSON blobs while serving (see
//! [`default_renderer`](crate::prelude::default_renderer)), and
//! [`ReactivityScript`] ships the runtime that hydrates and drives them, with no
//! WASM. Both gate on a live server ([`KeepAlive`](beet_net::prelude::KeepAlive)),
//! so a one-shot render and static export stay non-reactive.

mod reactivity_script;
pub use reactivity_script::*;
