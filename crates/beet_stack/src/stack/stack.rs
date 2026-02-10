use beet_core::prelude::*;

/// A single content container, similar to pages in a website or cards
/// in HyperCard. Each card is a route, with the exact rendering behavior
/// determined by the interface.
///
/// Cards may contain content, tools, and nested cards. The root entity
/// is automatically considered a card.
///
/// Use the [`card`] function to create a card with a path:
/// ```
/// use beet_stack::prelude::*;
/// use beet_core::prelude::*;
///
/// let mut world = StackPlugin::world();
/// let root = world.spawn((Card, children![
///     card("about"),
///     card("settings"),
/// ])).flush();
///
/// let tree = world.entity(root).get::<RouteTree>().unwrap();
/// tree.find_card(&["about"]).xpect_some();
/// tree.find_card(&["settings"]).xpect_some();
/// ```
#[derive(Component)]
pub struct Card;

/// Creates a card bundle with a [`PathPartial`] for routing.
///
/// This is the preferred way to create a card with a route path,
/// combining [`Card`] with a [`PathPartial`].
///
/// # Example
///
/// ```
/// use beet_stack::prelude::*;
/// use beet_core::prelude::*;
///
/// let mut world = StackPlugin::world();
/// let root = world.spawn((Card, children![
///     card("about"),
/// ])).flush();
///
/// let tree = world.entity(root).get::<RouteTree>().unwrap();
/// tree.find_card(&["about"]).xpect_some();
/// ```
pub fn card(path: &str) -> impl Bundle { (Card, PathPartial::new(path)) }
