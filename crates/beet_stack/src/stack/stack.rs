use beet_core::prelude::*;

/// A collection of cards organized as a stack.
///
/// A stack is the primary organizational unit for content in beet_stack.
/// Similar to how a website has pages, a stack contains multiple cards.
/// The stack itself can also be a card, acting like an index or home page.
///
/// # Example Structure
///
/// ```text
/// Stack (root)
///   ├─ Card (index/home)
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
