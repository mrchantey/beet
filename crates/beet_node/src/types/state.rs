use beet_core::prelude::*;


#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ElementState {
	Hovered,
	Focused,
	Pressed,
	Dragged,
	Disabled,
	Selected,
	Custom(SmolStr),
}


#[derive(Component, Deref, DerefMut)]
pub struct ElementStateMap(HashSet<ElementState>);
