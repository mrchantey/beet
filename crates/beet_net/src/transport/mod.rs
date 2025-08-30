mod event_source;
#[cfg(all(feature = "reqwest", not(target_arch = "wasm32")))]
mod impl_reqwest;
#[cfg(target_arch = "wasm32")]
mod impl_web_sys;
mod send;
