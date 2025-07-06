use crate::prelude::*;
use bevy::prelude::*;

#[derive(Default, Component, Reflect)]
#[reflect(Default, Component)]
#[require(StatProvider)]
pub struct ZoneStat {}
