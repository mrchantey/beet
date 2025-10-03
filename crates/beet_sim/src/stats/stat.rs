use crate::prelude::*;
use beet_core::prelude::*;
use std::fmt;
use std::ops::Range;


/// Stats are continuous values that represent real world phenomena.
/// For example they can be used to model health or frequency of events.
/// We deliberately use floating point because it more accurately represents
/// the continuous nature of the real world, and is required for utility ai.
#[derive(
	Component,
	Reflect,
	Debug,
	Default,
	Clone,
	Copy,
	PartialEq,
	PartialOrd,
	Deref,
	DerefMut,
)]
pub struct StatValue(pub f32);
impl Into<StatValue> for f32 {
	fn into(self) -> StatValue { StatValue(self) }
}
impl From<StatValue> for f32 {
	fn from(value: StatValue) -> f32 { value.0 }
}

impl fmt::Display for StatValue {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "{:.2}", self.0)
	}
}

impl StatValue {
	pub fn new(value: f32) -> Self { Self(value) }

	pub fn find_by_id(
		entity: Entity,
		children: Query<&Children>,
		stats: Query<(&StatId, &StatValue)>,
		id: StatId,
	) -> Option<StatValue> {
		let Some(children) = children.get(entity).ok() else {
			return None;
		};
		for child in children.iter() {
			if let Ok((stat_id, value)) = stats.get(child) {
				if *stat_id == id {
					return Some(*value);
				}
			}
		}
		None
	}

	pub fn range(range: Range<f32>) -> Range<StatValue> {
		StatValue(range.start)..StatValue(range.end)
	}


	pub fn normalize(&self, range: Range<StatValue>) -> f32 {
		(self.0 - *range.start) / (*range.end - *range.start)
	}
}


pub fn stat_plugin(app: &mut App) {
	app.register_type::<StatValue>()
		.world_mut()
		.register_component_hooks::<StatValue>()
		.on_add(|mut world, cx| {
			let map = world.resource::<StatMap>();
			let stat_id = world
				.get::<StatId>(cx.entity)
				.expect("StatValue requires StatId");

			let hexcode = map
				.get(stat_id)
				.expect("StatId must be in StatMap")
				.emoji_hexcode
				.clone();

			// world
			// 	.commands()
			// 	.entity(entity)
			// .insert(Emoji::bundle(&hexcode));
		});
}
