use std::fmt::Debug;
use std::hash::Hash;

pub trait Environment: Default {}

// pub trait DiscreteSpace {
// 	fn n(&self) -> usize;
// }
pub trait Space {
	type Value;
	const LEN: usize;
	fn shape(&self) -> SpaceShape;
	fn len(&self) -> usize { Self::LEN }
	fn sample(&self) -> Self::Value;
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum SpaceShape {
	Discrete(usize),
	// Box(Vec<usize>),
}

pub trait GridSpace: Space<Value = usize> {
	fn width(&self) -> usize;
	fn height(&self) -> usize;
}


pub trait ObservationSpace: Space + Debug + Hash + Eq + Clone {}


pub trait ActionSpace: Space {
	// fn sample(&self) -> usize;
}

pub trait RlEnvironment {
	fn observation_space(&self) -> impl ObservationSpace;
	fn action_space(&self) -> impl ActionSpace;

	// fn reset(&mut self);
}
