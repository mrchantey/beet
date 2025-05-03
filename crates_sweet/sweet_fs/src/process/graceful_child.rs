use std::process::Child;
use std::sync::Arc;
use std::sync::Mutex;



/// A child process holder that will actually kill the child process
/// if its dropped or ctrl-c is pressed
#[derive(Default, Clone)]
pub struct GracefulChild {
	pub process: Arc<Mutex<Option<Child>>>,
	/// dont announce that the child process is being killed
	pub print_on_exit: bool,
}

impl GracefulChild {
	/// set the [`ctrlc::set_handler`] to kill the child process
	pub fn as_only_ctrlc_handler(self) -> GracefulChild {
		let self2 = self.clone();
		ctrlc::set_handler(move || {
			self2.kill();
			std::process::exit(0);
		})
		.ok();
		self
	}

	/// Kills the previous child and sets the new one
	/// ## Panics
	/// - if the mutex is poisoned
	pub fn set(&self, new_process: Child) {
		self.kill();
		*self.process.lock().unwrap() = Some(new_process);
	}

	/// Kills the child process if its running
	/// ## Panics
	/// - if the mutex is poisoned
	pub fn kill(&self) {
		if let Some(mut process) = self.process.lock().unwrap().take() {
			let result = process.kill();
			if self.print_on_exit {
				println!("Child Process Killed: {:?}", result);
			}
		}
	}
}


impl Drop for GracefulChild {
	fn drop(&mut self) { self.kill(); }
}
