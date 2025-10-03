use beet_core::prelude::*;

/// Temp workaround since handle:Component removed in 0.15.
/// revisit with construct
#[derive(Debug, Clone, Component, Reflect, Deref, DerefMut)]
#[reflect(Component)]
pub struct HandleWrapper<T: Asset>(pub Handle<T>);
