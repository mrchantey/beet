use beet_core::prelude::*;

/// A collection of cards organized as a stack.
///
/// A stack is the primary organizational unit for content in beet_stack.
/// Similar to how a website has pages, a stack contains multiple cards.
/// The stack itself is also a card, by default presenting its cards as
/// navigation items, but it can have additional content and cards
/// just like other cards.
///
/// # Example Structure
///
/// ```text
/// Stack (root)
///   ├─ Card (page 1)
///   ├─ Card (page 2)
///   └─ Card (page 3)
/// ```
#[derive(Component)]
pub struct Stack;

/// A single content container within a [`Stack`].
///
/// Cards are the fundamental content units in beet_stack, similar to pages
/// in a website or cards in HyperCard. Each card contains content and tools.
///
/// Cards can also have associated [`Document`](crate::Document) components for
/// structured data storage.
#[derive(Component)]
pub struct Card;
