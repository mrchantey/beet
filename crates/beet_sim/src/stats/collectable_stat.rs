use bevy::prelude::*;

//// Superset of [`CollectableStat`], [`ZoneStat`] etc
#[derive(Default, Component, Reflect)]
#[reflect(Default, Component)]
pub struct StatProvider;

#[derive(Default, Component, Reflect)]
#[reflect(Default, Component)]
#[require(StatProvider)]
pub struct CollectableStat {}
