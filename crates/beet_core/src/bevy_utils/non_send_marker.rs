use bevy::prelude::*;


/// A dummy type that is [`!Send`](Send), to force systems to run on the main thread.
// TODO use bevy's marker in 0.16.2
pub type TempNonSendMarker<'w> = Option<NonSend<'w, ()>>;


// /// A component with the inner wrapped in a [`SendWrapper`].
// /// ## Panics
// /// Any system that accesses this component must be run on the main thread.
// /// Use [`NonSendMarker`] in every system where this is used.
// ///
// /// ## Example
// /// ```rust
// /// # use bevy::prelude::*;
// /// # use beet_core::prelude::*;
// ///
// /// fn my_system(
// /// 	_: TempNonSendMarker,
// /// 	query: Query<&NonSendComp<RefCell<usize>>>){
// /// }
// ///
// #[derive(Debug, Clone, Deref, DerefMut, Component)]
// pub struct NonSendComp<T>(pub SendWrapper<T>);
// impl<T> NonSendComp<T> {
// 	pub fn new(value: T) -> Self { Self(SendWrapper::new(value)) }
// 	pub fn take(self) -> T { self.0.take() }
// }
