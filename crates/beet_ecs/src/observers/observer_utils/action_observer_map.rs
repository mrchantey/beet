use bevy::prelude::*;
use bevy::utils::HashSet;



#[derive(Debug, Default, Component)]
pub struct ActionObserverMap<T> {
	pub observers: HashSet<Entity>,
	pub _phantom: std::marker::PhantomData<T>,
}

impl<T> ActionObserverMap<T> {
	pub fn new(observers: impl IntoIterator<Item = Entity>) -> Self {
		Self {
			observers: observers.into_iter().collect(),
			_phantom: Default::default(),
		}
	}
}

impl<T: 'static + Send + Sync> ActionObserverMap<T> {
	
}

impl<T> Clone for ActionObserverMap<T> {
	fn clone(&self) -> Self {
		Self {
			observers: self.observers.clone(),
			_phantom: Default::default(),
		}
	}
}
