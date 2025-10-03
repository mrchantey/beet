use crate::prelude::*;
use beet_core::prelude::*;

#[derive(Default, Component, Reflect)]
#[reflect(Default, Component)]
#[require(StatProvider)]
pub struct ZoneStat {}
