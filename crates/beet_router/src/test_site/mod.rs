mod components;
pub use components::*;
// #[path = "routes/docs/index.rs"]
// mod foo;
#[path = "codegen/routes.rs"]
pub mod routes;
use crate::as_beet::*;
