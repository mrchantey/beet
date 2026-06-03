// the serde-marshalled `Script`/`ScriptAction` need serde; the rhai serde
// runtime additionally needs std (via `rhai_serde`). Without serde the agnostic
// layer is just `ScriptLanguage`; without `rhai_serde` `Script::run` has no rhai
// backend.
#[cfg(feature = "rhai_serde")]
mod rhai_runtime;
#[cfg(feature = "serde")]
mod script;
#[cfg(feature = "serde")]
mod script_action;
#[cfg(feature = "rhai_serde")]
pub(crate) use rhai_runtime::run_rhai;
#[cfg(feature = "serde")]
pub use script::*;
#[cfg(feature = "serde")]
pub use script_action::*;
