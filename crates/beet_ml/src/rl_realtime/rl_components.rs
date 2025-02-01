#[allow(unused)]
use crate::prelude::*;
use bevy::prelude::*;

#[derive(Component)]
pub struct Reward(pub f32);

#[derive(Component)]
pub struct Episode(pub f32);
