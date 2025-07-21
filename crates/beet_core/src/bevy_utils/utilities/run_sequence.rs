use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;
use variadics_please::all_tuples;


/// abstraction so impl WorldSequence only needs to implement one method
pub trait WorldSequenceRunner {
	fn world(&self) -> &World;
	fn world_mut(&mut self) -> &mut World;
	fn run<T, O, M>(&mut self, system: T) -> Result<()>
	where
		T: 'static + IntoSystem<(), O, M>,
		O: WorldSequenceOutput;
}

pub trait WorldSequenceOutput: 'static {
	fn maybe_result(self) -> Result<()>;
}
impl WorldSequenceOutput for () {
	fn maybe_result(self) -> Result<()> { Ok(()) }
}
impl WorldSequenceOutput for Result<()> {
	fn maybe_result(self) -> Result<()> { self }
}

pub(crate) struct RunOnce<'a>(pub &'a mut World);
impl WorldSequenceRunner for RunOnce<'_> {
	fn world(&self) -> &World { self.0 }
	fn world_mut(&mut self) -> &mut World { self.0 }
	fn run<T, O, M>(&mut self, system: T) -> Result<()>
	where
		T: 'static + IntoSystem<(), O, M>,
		O: WorldSequenceOutput,
	{
		match self.0.run_system_once(system) {
			Ok(out) => out.maybe_result(),
			// ignore the 'runsystemerror', ie empty Populated
			Err(_) => Ok(()),
		}
	}
}
pub(crate) struct RunCached<'a>(pub &'a mut World);
impl WorldSequenceRunner for RunCached<'_> {
	fn world(&self) -> &World { self.0 }
	fn world_mut(&mut self) -> &mut World { self.0 }
	fn run<T, O, M>(&mut self, system: T) -> Result<()>
	where
		T: 'static + IntoSystem<(), O, M>,
		O: WorldSequenceOutput,
	{
		match self.0.run_system_cached(system) {
			Ok(out) => out.maybe_result(),
			// ignore the 'runsystemerror', ie empty Populated
			Err(_) => Ok(()),
		}
	}
}


pub trait WorldSequence<M = Self>: Sized {
	fn run_sequence<R: WorldSequenceRunner>(self, runner: &mut R)
	-> Result<()>;
}

pub struct TupleWorldSequenceMarker;

// Variadic macro for WorldSequence
#[allow(non_snake_case)]
macro_rules! impl_run_systems_tuple {
	($(($T:ident, $O:ident, $M:ident, $t:ident)),*) => {
		#[allow(non_snake_case)]
		#[allow(non_camel_case_types)]
		impl<$($T, $O, $M),*> WorldSequence<(TupleWorldSequenceMarker, $($O, $M,)*)> for ($($T,)*)
		where
			$($T: 'static + IntoSystem<(), $O, $M>, $O: WorldSequenceOutput,)*
		{
			fn run_sequence<R: WorldSequenceRunner>(self, runner: &mut R) -> Result<()> {
				let ($($t,)*) = self;
				$(
					runner.run($t)?;
				)*
				Ok(())
			}
		}
	}
}
all_tuples!(impl_run_systems_tuple, 1, 16, M, O, T, t);

pub struct IntoWorldSequenceMarker;

impl<T, O, M> WorldSequence<(IntoWorldSequenceMarker, O, M)> for T
where
	T: 'static + IntoSystem<(), O, M>,
	O: WorldSequenceOutput,
{
	fn run_sequence<R: WorldSequenceRunner>(
		self,
		runner: &mut R,
	) -> Result<()> {
		runner.run(self)
	}
}




#[cfg(test)]
mod test {
	use crate::prelude::*;
	use bevy::prelude::*;
	use std::sync::Arc;
	use std::sync::Mutex;
	use sweet::prelude::*;

	#[test]
	fn works() {
		let mut world = World::new();
		let counter = Arc::new(Mutex::new(0));

		let counter_a = counter.clone();
		let system_a = move || {
			let mut guard = counter_a.lock().unwrap();
			*guard += 1;
		};

		let counter_b = counter.clone();
		let system_b = move || -> Result<()> {
			let mut guard = counter_b.lock().unwrap();
			*guard += 1;
			Ok(())
		};
		// invalid system param
		let counter_c = counter.clone();
		let system_c = move |_: When<Res<Time>>| -> Result<()> {
			let mut guard = counter_c.lock().unwrap();
			*guard += 1;
			Ok(())
		};
		world
			.run_sequence_once((system_a, system_b, system_c))
			.unwrap();

		let value = *counter.lock().unwrap();
		expect(value).to_be(2);
	}
}
