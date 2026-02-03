use beet_core::prelude::*;






/// A stack is a [`Card`] collection. The stack itself may
/// be a card, in which case it behaves somewhat like an `index.html`
#[derive(Component)]
pub struct Stack;



/// A card is a single content container within a [`Stack`]
#[derive(Component)]
pub struct Card;
