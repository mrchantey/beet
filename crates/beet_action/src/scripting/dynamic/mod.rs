//! The world bridge: how a sandboxed [`Script`](crate::prelude::Script) reaches
//! the [`World`](bevy::prelude::World) without holding any authority over it.
//!
//! A script's `world` global is a channel, not the world: every method on it is
//! a *request*: the script sends a [`WorldCall`] and awaits a promise, the host
//! performs the operation against the live world and answers with a
//! [`WorldReply`], and that reply settles the promise.
//!
//! So world access is served live rather than mediated in phases. A read
//! returns the world as it is at the moment of the call, a write lands
//! immediately and in order, a `spawn` resolves to a real entity id the next
//! line can use, and a refused call rejects where it was made, catchable by the
//! script. What the script never holds is the world itself: it holds a channel,
//! and the host decides what crosses it.
//!
//! Every backend serves through one [`WorldBridge`], and every operation is
//! async: it takes exclusive world access for as long as it needs and gives it
//! back, so a check that is legitimately asynchronous runs with nothing held.
//! [`WorldRead`] and [`WorldWrite`] are the synchronous `&mut World` halves
//! those sections call.
//!
//! ## Reach
//!
//! A world-capable script may address any component by default. A scene running
//! a script it trusts less narrows its [`ScriptConfig`], and the bridge, not the
//! script, enforces it per call: reads check the read filter, mutations check
//! the write filter, and a small set of components (the config itself and the
//! script carriers) is refused unconditionally, so a script can never widen its
//! own grant. Withholding the world entirely is the same config's `world: false`,
//! which leaves no `world` global to reach for at all.
//!
//! ## Vocabulary
//!
//! [`DynamicComponent`] mints a component type with no rust definition behind
//! it, holding a [`Value`](beet_core::prelude::Value), so a scene can add words
//! the engine never shipped and a script can read and write them exactly as it
//! does a registered one. Its
//! [`ValueSchema`](beet_core::prelude::ValueSchema) is what the word *means*:
//! open by default, and once declared, every write is validated against it
//! before the value reaches the component's storage, so a rejection reaches the
//! script as the same catchable error a refusal is.

// the wire form of an entity, one pair of helpers so the format lives in one
// place.
pub mod entity_id;

mod component_ident;
mod dynamic_component;
mod world_bridge;
mod world_call;
mod world_read;
mod world_write;
pub use component_ident::*;
pub use dynamic_component::*;
pub use world_bridge::*;
pub use world_call::*;
pub use world_read::*;
pub use world_write::*;

// The JS half is only meaningful where something evaluates it, and both the
// embedded engine and the host-realm runner splice it, so it lives beside the
// data types and stays crate-internal.
mod world_shim;
pub(crate) use world_shim::WORLD_SHIM;

#[cfg(test)]
pub(crate) mod test_support;
