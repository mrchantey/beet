use crate::prelude::*;
use bevy_derive::Deref;
use bevy_derive::DerefMut;
use bevy_ecs::prelude::*;


// #[action(
// 	system=constant_score,
// 	set=PreTickSet,
// 	components=Score::default()
// )]
#[derive(Default, Deref, DerefMut, Action)]
#[action(set=PreTickSet)]
pub struct ConstantScore(pub Score);

impl ConstantScore {
	pub fn new(score: Score) -> Self { Self(score) }
}

pub fn constant_score(
	mut query: Query<(&ConstantScore, &mut Score), Added<ConstantScore>>,
) {
	for (from, mut to) in query.iter_mut() {
		*to = **from;
	}
}
