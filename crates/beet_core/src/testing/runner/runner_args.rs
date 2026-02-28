//! Test runner arguments parsed from CLI input.
//!
//! [`TestRunnerArgs`] replaces `Request`/`RequestMeta` in the testing context,
//! providing a lightweight component for passing CLI arguments to the test runner
//! without depending on the exchange module.

use crate::prelude::*;
use bevy::reflect::FromReflect;
use bevy::reflect::Typed;

/// CLI arguments parsed into a form usable by the test runner.
///
/// This is a [`Component`] spawned alongside test bundles, containing
/// the parsed path and params from CLI input.
#[derive(Debug, Clone, Component)]
pub struct TestRunnerArgs {
	/// Positional arguments forming the path.
	path: Vec<String>,
	/// Named arguments as key-value pairs.
	params: MultiMap<String, String>,
	/// The instant this was created, for timing.
	started: Instant,
}

impl TestRunnerArgs {
	/// Creates args from a [`CliArgs`] instance.
	pub fn from_cli_args(args: CliArgs) -> Self {
		let mut params = MultiMap::new();
		for (key, values) in args.params {
			if values.is_empty() {
				params.insert_key(key);
			} else {
				for value in values {
					params.insert(key.clone(), value);
				}
			}
		}
		Self {
			path: args.path,
			params,
			started: Instant::now(),
		}
	}

	/// Creates args by parsing a CLI-style string.
	pub fn from_cli_str(args: &str) -> Self {
		Self::from_cli_args(CliArgs::parse(args))
	}

	/// Creates args from environment CLI arguments.
	pub fn from_env() -> Self { Self::from_cli_args(CliArgs::parse_env()) }

	/// Returns the positional path arguments.
	pub fn path(&self) -> &Vec<String> { &self.path }

	/// Returns the named parameters.
	pub fn params(&self) -> &MultiMap<String, String> { &self.params }

	/// Returns the instant this was created.
	pub fn started(&self) -> Instant { self.started }
}

/// A system parameter for extracting typed params from [`TestRunnerArgs`],
/// with caching via component insertion.
#[derive(SystemParam)]
pub struct TestParamQuery<'w, 's, T: Component> {
	/// Commands for inserting cached components.
	pub commands: Commands<'w, 's>,
	/// Query for accessing runner args and cached params.
	pub agents: Query<'w, 's, (&'static TestRunnerArgs, Option<&'static T>)>,
}

impl<T: Clone + Component> TestParamQuery<'_, '_, T> {
	/// Extracts the param from the runner args, caching the result as a component.
	pub fn get(&mut self, agent: Entity) -> Result<T>
	where
		T: Sized + Clone + FromReflect + Typed + Component,
	{
		self.get_custom(agent, |args| args.params().parse_reflect::<T>())
	}

	/// Extracts the param using a custom function, caching the result.
	pub fn get_custom<F>(&mut self, agent: Entity, func: F) -> Result<T>
	where
		F: FnOnce(&TestRunnerArgs) -> Result<T>,
	{
		let (args, params) = self.agents.get(agent)?;
		match params {
			Some(params) => Ok(params.clone()),
			None => {
				let params = func(args)?;
				self.commands.entity(agent).insert(params.clone());
				Ok(params)
			}
		}
	}
}
