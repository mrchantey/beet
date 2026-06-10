use beet_core::prelude::*;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum ElementState {
	Hovered,
	Focused,
	Pressed,
	Dragged,
	Disabled,
	Selected,
	Custom(SmolStr),
}

/// The active interactive states ([`ElementState`]) on an element, the set
/// `:hover`/`:focus`/`:disabled`/... selectors match against.
#[derive(Default, Component, Deref, DerefMut)]
pub struct ElementStateMap(HashSet<ElementState>);

impl ElementStateMap {
	/// An empty state map.
	pub fn new() -> Self { Self::default() }

	/// A state map carrying a single `state`.
	pub fn with(state: ElementState) -> Self {
		let mut map = Self::default();
		map.insert(state);
		map
	}
}
