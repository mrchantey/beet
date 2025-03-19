use crate::prelude::*;
use anyhow::Result;
use beet::prelude::*;

pub trait BuildStep: 'static + Send + Sync {
	fn run(&self) -> Result<()>;
}

impl BuildStep for BuildStepGroup {
	fn run(&self) -> Result<()> {
		for item in &self.items {
			item.run()?;
		}
		Ok(())
	}
}


impl BuildStep for CollectRoutes {
	fn run(&self) -> Result<()> {
		self.build_and_write()?;
		Ok(())
	}
}


impl<T> BuildStep for T
where
	T: 'static + Send + Sync + Fn() -> Result<()>,
{
	fn run(&self) -> Result<()> { self() }
}

impl BuildStep for BuildCmd {
	fn run(&self) -> Result<()> {
		self.spawn()?;
		Ok(())
	}
}


#[derive(Default)]
pub struct BuildStepGroup {
	pub items: Vec<Box<dyn BuildStep>>,
}

impl BuildStepGroup {
	pub fn add(&mut self, item: impl BuildStep + 'static) -> &mut Self {
		self.items.push(Box::new(item));
		self
	}
}
