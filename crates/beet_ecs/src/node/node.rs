use bevy_derive::Deref;
use bevy_derive::DerefMut;
use bevy_ecs::prelude::*;

#[derive(Debug, PartialEq, Deref, DerefMut, Component)]
pub struct TargetEntity(pub Entity);
