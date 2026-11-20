//! [`ForeignChildren`]: an element whose children the browser fills in.
use beet_core::prelude::*;

/// Marks an element whose children the document fills in once served, ie a
/// server island's `<beet-island>` wrapper, whose inline script swaps the
/// island's html in over the fallback the world renders.
///
/// The world's children are the first paint only. The DOM sink adopts a
/// served element whole, binding it without walking, patching or binding
/// inside it, since what it holds may already be the document's; the
/// incremental pass never reconciles inside one; and a fresh paint, which
/// runs no script, paints the world's children and announces it
/// (`ForeignChildrenPainted`), so whoever owns the fill runs it. Every other
/// sink renders the children as any element's.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Component, Reflect)]
#[reflect(Component, Default)]
pub struct ForeignChildren;
