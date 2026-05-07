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


#[derive(Component, Deref, DerefMut)]
pub struct ElementStateMap(HashSet<ElementState>);
