use crate::prelude::*;
use beet_core::prelude::*;

impl Action<(), ()> {
	/// Create an [`Action`] that runs a [`Script`] against the caller entity.
	///
	/// The script executes when the action is called, with the caller's
	/// reflected components exposed for reading and mutation.
	pub fn new_script(script: Script) -> Self {
		Action::new(
			TypeMeta::of::<Script>(),
			move |ActionCall {
			          mut commands,
			          caller,
			          input: (),
			          out_handler,
			      }| {
				let script = script.clone();
				commands.commands.queue(move |world: &mut World| -> Result {
					let result = script.run(world, caller);
					out_handler.call_world(world, result)
				});
				Ok(())
			},
		)
	}
}

/// Marker for the [`IntoAction`] impl on [`Script`].
pub struct ScriptActionMarker;

impl IntoAction<ScriptActionMarker> for Script {
	type In = ();
	type Out = ();

	fn into_action(self) -> Action<(), ()> { Action::new_script(self) }
}
