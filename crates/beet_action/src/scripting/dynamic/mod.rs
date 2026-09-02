//! The world bridge: how a sandboxed [`Script`](crate::prelude::Script) reaches
//! the [`World`](bevy::prelude::World) without holding any authority over it.
//!
//! A `Script` on its own is a pure `Input -> Output` transform with no `world`
//! global at all. A bridged evaluation adds one, and every method on it is a
//! *request*: the script sends a [`WorldCall`] and awaits a promise, the host
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
//! ## Exposure
//!
//! A world-bridged script may address any component by default. A scene running
//! a script it trusts less declares a [`ScriptExposure`], and the bridge, not
//! the script, enforces it per call: reads check the read filter, mutations
//! check the write filter, and a small set of components ([`ScriptExposure`]
//! itself and the script carriers) is refused unconditionally, so a script can
//! never widen its own grant. Sandboxed by default in authority, restrictable
//! per script in reach.
//!
//! ## Vocabulary
//!
//! [`DynamicComponent`] mints a component type with no rust definition behind
//! it, holding a [`Value`](beet_core::prelude::Value), so a scene can add words
//! the engine never shipped and a script can read and write them exactly as it
//! does a registered one.

// the wire form of an entity, one pair of helpers so the format lives in one
// place.
pub mod entity_id;

mod component_ident;
mod dynamic_component;
mod dynamic_script;
mod script_exposure;
mod world_bridge;
mod world_call;
mod world_read;
mod world_write;
pub use component_ident::*;
pub use dynamic_component::*;
pub use dynamic_script::*;
pub use script_exposure::*;
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
