use crate::prelude::*;
use bevy::prelude::*;

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct Desire<T> {
	pub value: DesiredVaule<T>,
	pub stat_id: StatId,
}

#[derive(Debug, Default, Reflect)]
pub enum DesiredVaule<T> {
	Min,
	#[default]
	Max,
	Exact(T),
}
