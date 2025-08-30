mod cross_fetch;
#[cfg(all(feature = "reqwest", not(target_arch = "wasm32")))]
mod impl_reqwest;
#[cfg(target_arch = "wasm32")]
mod impl_web_sys;
mod event_source;
