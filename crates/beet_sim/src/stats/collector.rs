use crate::prelude::*;
use bevy::prelude::*;


/// Marker to indicate that an entity is a collector.
#[derive(Default, Component, Reflect)]
#[reflect(Default, Component)]
pub struct Collector;
