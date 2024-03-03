use crate::prelude::*;
use bevy_ecs::prelude::*;
use serde::Deserialize;
use serde::Serialize;
use std::cmp::Ordering;
use std::fmt::Debug;

/// Indicate this node's parent will use the scores in the next tick.
/// As this is frequently added and removed, it is `SparseSet`.
#[derive(Default, Debug, Component)]
#[component(storage = "SparseSet")]
pub struct Scoring;


/// Used to indicate to selectors how favorable a child node would be to run.
#[derive(
	Debug,
	Default,
	Serialize,
	Deserialize,
	Clone,
	Copy,
	Component,
	PartialEq,
	FieldUi,
	// strum
	strum_macros::Display,
	strum_macros::EnumIter,
)]
pub enum Score {
	#[default]
	/// The node should not run.
	Fail,
	/// The node has a weight in the range `0..100`, where higher is more favorable.
	Weight(u8),
	// Weight(#[number(min = 0, max = 100, step = 1)] u8),
	/// The node should run.
	Pass,
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
			// (Score::Weight(w1), Score::Weight(w2)) => w1.total_cmp(&w2),
			(Score::Weight(w1), Score::Weight(w2)) => w1.cmp(&w2),
		};
		Some(val)
	}
}
