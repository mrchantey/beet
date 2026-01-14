mod event_source;
#[cfg(all(feature = "reqwest", not(target_arch = "wasm32")))]
mod impl_reqwest;
#[cfg(all(feature = "ureq", not(target_arch = "wasm32")))]
mod impl_ureq;
#[cfg(target_arch = "wasm32")]
mod impl_web_sys;
// pub use event_source::*;
mod send;
pub use event_source::*;
pub use send::*;
