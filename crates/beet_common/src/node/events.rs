use bevy::prelude::*;

#[derive(Debug, Clone, Reflect, Event)]
pub struct OnClick;



#[derive(Default, Component, Reflect)]
#[reflect(Default, Component)]
pub struct EventObserver {
	/// The unchanged event name used in the template, which
	/// may be one of several casings, ie
	/// `onmousemove`, `onMouseMove`, `OnMouseMove`
	name: String,
}

impl EventObserver {
	/// Create a new event observer with the given name
	pub fn new(name: &str) -> Self {
		Self {
			name: name.to_string(),
		}
	}

	/// Get the event name in a consistent lowercase format
	pub fn event_name(&self) -> String { self.name.to_lowercase() }
}
