use beet_core::prelude::*;
use bevy::ecs::component::Mutable;
use std::fmt::Debug;
use std::hash::Hash;

#[derive(Deref)]
pub struct Readonly<T>(T);
impl<T> Readonly<T> {
	pub fn new(value: T) -> Self { Self(value) }
}


pub trait Environment: 'static + Send + Sync + Clone {
	type State: StateSpace;
	type Action: ActionSpace;

	fn step(
		&mut self,
		state: &Self::State,
		action: &Self::Action,
	) -> StepOutcome<Self::State>;
	// fn state_space(&self) -> State;
	// fn action_space(&self) -> Action;
}

#[derive(Clone)]
pub struct StepOutcome<State> {
	pub state: State,
	pub reward: f32,
	pub done: bool,
}

pub trait DiscreteSpace:
	'static
	+ Send
	+ Sync
	+ Debug
	+ Hash
	+ Clone
	+ PartialEq
	+ Eq
	+ Component<Mutability = Mutable>
	+ TypePath
{
	// type Value;
	// const LEN: usize;
	// // fn shape(&self) -> SpaceShape;
	// fn len(&self) -> usize { Self::LEN }
	// fn sample(&self) -> Self::Value;
}
impl<
	T: 'static
		+ Send
		+ Sync
		+ Debug
		+ Hash
		+ Clone
		+ PartialEq
		+ Eq
		+ Component<Mutability = Mutable>
		+ TypePath,
> DiscreteSpace for T
{
}


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
