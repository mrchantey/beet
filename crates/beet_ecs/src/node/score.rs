use crate::prelude::ReflectInspectorOptions;
use beet_ecs_macros::InspectorOptions;
use bevy::prelude::*;
use std::cmp::Ordering;
use std::fmt::Debug;

// TODO
/// Indicate this node's parent will use the scores in the next tick.
/// As this is frequently added and removed, it is `SparseSet`.
#[derive(Default, Debug, Component)]
#[component(storage = "SparseSet")]
pub struct Scoring;


/// Score is a primitive of [`beet`]. The weight is almost always in the range of `0..1` Like a [`Vec3`], the meaning of a [`Score`] depends on its context, for example:
/// - As a parameter of an Astar cost component
/// - Indicate to selectors how favorable a child node would be to run.
#[derive(
	Debug,
	Default,
	Clone,
	Copy,
	Component,
	PartialEq,
	// strum
	strum_macros::Display,
	strum_macros::EnumIter,
	Reflect,
	InspectorOptions,
)]
#[reflect(Component, InspectorOptions)]
pub enum Score {
	#[default]
	/// Lowest possible score, ie the node should not run.
	Fail,
	Weight(#[inspector(min = 0., max = 1., step = 0.01)] f32),
	/// The node has a weight, usually in the range `0..1`, where higher is more favorable.
	// Weight(#[number(min = 0, max = 100, step = 1)] u8),
	/// Highest possible score, ie the node should run.
	Pass,
}

impl Score {
	/// Maps [`Score::Fail`] to `0.0`, [`Score::Pass`] to `1.0` and [`Score::Weight`] to its value.
	pub fn weight(&self) -> f32 {
		match self {
			Score::Fail => 0.0,
			Score::Weight(w) => *w,
			Score::Pass => 1.0,
		}
	}
}

impl PartialOrd for Score {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		let val = match (self, other) {
			(Score::Fail, Score::Fail) => Ordering::Equal,
			(Score::Fail, _) => Ordering::Less,
			(_, Score::Fail) => Ordering::Greater,
			(Score::Pass, Score::Pass) => Ordering::Equal,
			(Score::Pass, _) => Ordering::Greater,
			(_, Score::Pass) => Ordering::Less,
			(Score::Weight(w1), Score::Weight(w2)) => w1.total_cmp(&w2),
			// (Score::Weight(w1), Score::Weight(w2)) => w1.cmp(&w2),
		};
		Some(val)
	}
}



#[cfg(test)]
mod test {
	use crate::prelude::*;
	use anyhow::Result;
	use sweet::*;

	#[test]
	fn works() -> Result<()> {
		expect(Score::Fail).to_be(Score::Fail)?;
		expect(Score::Fail).to_be_less_than(Score::Pass)?;
		expect(Score::Fail).to_be_less_than(Score::Weight(0.5))?;
		expect(Score::Weight(0.5)).to_be_less_than(Score::Pass)?;
		expect(Score::Weight(0.4)).to_be_less_than(Score::Weight(0.5))?;

		Ok(())
	}
}
