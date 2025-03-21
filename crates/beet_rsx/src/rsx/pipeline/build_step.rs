use anyhow::Result;

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

impl<T> BuildStep for T
where
	T: 'static + Send + Sync + Fn() -> Result<()>,
{
	fn run(&self) -> Result<()> { self() }
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
