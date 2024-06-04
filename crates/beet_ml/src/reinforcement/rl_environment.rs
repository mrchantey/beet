use std::fmt::Debug;
use std::hash::Hash;

pub trait Environment<State: StateSpace, Action: ActionSpace> {
	fn state(&self) -> State;
	fn step(&mut self, action: Action) -> StepOutcome<State>;
	// fn state_space(&self) -> State;
	// fn action_space(&self) -> Action;
}

pub struct StepOutcome<State> {
	pub state: State,
	pub reward: f32,
	pub done: bool,
}

pub trait Space: Debug + Hash + Clone {
	// type Value;
	// const LEN: usize;
	// // fn shape(&self) -> SpaceShape;
	// fn len(&self) -> usize { Self::LEN }
	// fn sample(&self) -> Self::Value;
}
impl<T: Debug + Hash + Clone> Space for T {}



// #[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
// pub enum SpaceShape {
// 	Discrete(usize),
// 	// Box(Vec<usize>),
// }

pub trait StateSpace: Space + Into<usize> {}
impl<T: Space + Into<usize>> StateSpace for T {}

pub trait ActionSpace: Space + From<usize> {}
impl<T: Space + From<usize>> ActionSpace for T {}
