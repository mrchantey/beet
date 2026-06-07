//! The site's document layout and navigation, shared by the web and terminal
//! render targets.
//!
//! [`BeetLayout`] is the global layout fed to
//! [`Layout`](beet::prelude::Layout): it receives each route's
//! rendered body as its `children` and places it in `<main>`. The web-only
//! pieces (the built stylesheet, color-scheme seed, preflight reset, favicon
//! link) live in the document `<head>`, which is non-visual and does not paint
//! in the terminal, so the one layout serves both targets.
mod layout;
pub use layout::*;
mod sidebar;
pub use sidebar::*;
