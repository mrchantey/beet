//! The hand-authored controls a form is built from: [`Button`], [`Checkbox`],
//! the text/number/select fields and their [`Form`], the [`Error`] pair, and
//! [`Table`].
//!
//! These take authored props, not a schema — the generated counterparts live in
//! [`schema_ui`](super::schema_ui), which dispatches to these same controls.
//! When a bound control's edit reaches its document is its
//! [`WritePolicy`](beet_core::prelude::WritePolicy), enforced by
//! [`write_policy`].
pub(in crate::widgets) mod button;
mod checkbox;
pub(in crate::widgets) mod error;
mod form;
mod table;
pub(in crate::widgets) mod write_policy;

pub use button::*;
pub use checkbox::*;
pub use error::*;
pub use form::*;
pub use table::*;
