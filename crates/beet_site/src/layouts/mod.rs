//! The site's document shell and navigation, shared by the web and terminal
//! render targets.
//!
//! [`BeetDocumentShell`] is the global layout fed to
//! [`DocumentShell`](beet::prelude::DocumentShell): it receives each route's
//! rendered body as its `children` and places it in `<main>`. The web-only
//! pieces (the built stylesheet, color-scheme seed, preflight reset, favicon
//! link) live in the document `<head>`, which is non-visual and does not paint
//! in the terminal, so the one shell serves both targets.
mod head;
pub use head::*;
mod shell;
pub use shell::*;
mod sidebar;
pub use sidebar::*;
