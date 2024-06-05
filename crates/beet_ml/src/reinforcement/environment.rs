use rand::Rng;
use std::fmt::Debug;
use std::hash::Hash;

pub trait Environment {
	type State: StateSpace;
	type Action: ActionSpace;

	fn state(&self) -> Self::State;
	fn step(&mut self, action: &Self::Action) -> StepOutcome<Self::State>;
	// fn state_space(&self) -> State;
	// fn action_space(&self) -> Action;
}

pub struct StepOutcome<State> {
	pub state: State,
	pub reward: f32,
	pub done: bool,
}

pub trait DiscreteSpace: Debug + Hash + Clone + PartialEq + Eq {
	// type Value;
	// const LEN: usize;
	// // fn shape(&self) -> SpaceShape;
	// fn len(&self) -> usize { Self::LEN }
	// fn sample(&self) -> Self::Value;
}
impl<T: Debug + Hash + Clone + PartialEq + Eq> DiscreteSpace for T {}


// #[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
// pub enum SpaceShape {
// 	Discrete(usize),
// 	// Box(Vec<usize>),
// }

pub trait StateSpace: DiscreteSpace {}
impl<T: DiscreteSpace> StateSpace for T {}

pub trait ActionSpace: DiscreteSpace + Default {
	fn sample(rng: &mut impl Rng) -> Self;
}
// impl<T: DiscreteSpace + TryFrom<usize>> ActionSpace for T {}
