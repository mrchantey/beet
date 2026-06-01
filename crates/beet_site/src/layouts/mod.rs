//! The site's document shell and navigation, shared by the web and terminal
//! render targets.
//!
//! [`beet_document_shell`] is the global layout scene fed to
//! [`document_shell`](beet::prelude::document_shell): every route's rendered
//! body is slotted into its `<slot name="main">`. The web-only pieces (the
//! built stylesheet, color-scheme seed, preflight reset, favicon link) live in
//! the document `<head>`, which the charcell renderer skips, so the one shell
//! serves both targets.
mod head;
pub use head::*;
mod shell;
pub use shell::*;
mod sidebar;
pub use sidebar::*;
