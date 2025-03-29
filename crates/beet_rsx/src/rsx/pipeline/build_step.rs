use anyhow::Result;

pub trait BuildStep {
	fn run(&self) -> Result<()>;
}


impl<T> BuildStep for T
where
	T: Fn() -> Result<()>,
{
	fn run(&self) -> Result<()> { self() }
}



#[derive(Default)]
pub struct BuildStepGroup {
	pub items: Vec<Box<dyn 'static + Send + Sync + BuildStep>>,
}

impl BuildStepGroup {
	pub fn add(
		&mut self,
		item: impl BuildStep + 'static + Send + Sync,
	) -> &mut Self {
		self.items.push(Box::new(item));
		self
	}
}

impl BuildStep for BuildStepGroup {
	fn run(&self) -> Result<()> {
		for item in &self.items {
			item.run()?;
		}
		Ok(())
	}
}
