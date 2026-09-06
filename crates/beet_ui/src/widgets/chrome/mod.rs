//! Page chrome: the document [`Head`], the [`Header`]/[`Footer`] bars, the
//! [`PageLayout`]/[`ContentLayout`] wrappers, and the [`Sidebar`] rail.
//!
//! Target-neutral and route-unaware — a request-aware composition (which route
//! is active, what the nav tree holds) belongs to `beet_router`, which feeds
//! these widgets.
mod footer;
mod head;
mod header;
mod layout;
mod sidebar;

pub use footer::*;
pub use head::*;
pub use header::*;
pub use layout::*;
pub use sidebar::*;
