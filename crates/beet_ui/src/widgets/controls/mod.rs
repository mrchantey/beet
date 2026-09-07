//! The hand-authored controls a form is built from: [`Button`], [`Checkbox`],
//! the text/number/select fields and their [`Form`], the [`Error`] pair, and
//! [`Table`].
//!
//! These take authored props, not a schema — the generated counterparts live in
//! [`schema_ui`](super::schema_ui), which dispatches to these same controls.
pub(in crate::widgets) mod button;
mod checkbox;
pub(in crate::widgets) mod error;
mod form;
mod table;

pub use button::*;
pub use checkbox::*;
pub use error::*;
pub use form::*;
pub use table::*;
