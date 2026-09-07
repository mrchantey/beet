//! The url space and what runs in it, in four groups: the [`model`] a request is
//! matched against, the [`dispatch`] that runs it, the [`policy`] middleware
//! wrapped around it, and the [`site`] chrome a page route renders through.
//!
//! The no_std core is `model` + `dispatch` + `policy`: the route tree, path
//! patterns, standalone middleware, and the server-action client. `site` is
//! std-only throughout, being built on the `beet_ui` scene pipeline.

mod dispatch;
mod model;
mod policy;
mod site;

pub use dispatch::*;
pub use model::*;
pub use policy::*;
pub use site::*;
