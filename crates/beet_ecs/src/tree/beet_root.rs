use bevy::prelude::*;

/// Couldn't resist.. Marker to identify the root of a behavior graph
#[derive(Debug, Default, Component, Reflect)]
#[reflect(Component, Default)]
pub struct BeetRoot;