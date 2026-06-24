use crate::prelude::*;
use beet_core::prelude::*;

/// Load verb for a render scene: on its entity's [`LoadTemplate`] (delivered to
/// every node once that node's assets and scene have loaded, see
/// `drain_pending_dependencies`), kicks the entity's behaviour into `Running`.
///
/// The render-scene counterpart of the server's `BootOnLoad`: where `BootOnLoad`
/// calls an entry's `Action<Boot, Response>` slot, `RunOnLoad` starts the
/// behaviour tree itself. It replaces a `CallOnSpawn`-on-`Added` kick so a
/// behaviour never starts before its assets exist, and sits on any node whose
/// subtree should start on load (eg a scene's behaviour-tree root).
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component, Default)]
#[component(on_add = on_add_run)]
pub struct RunOnLoad;

/// Register the entity-scoped [`LoadTemplate`] observer, so the verb starts the
/// node it sits on once that node's own `LoadTemplate` fires.
fn on_add_run(mut world: DeferredWorld, cx: HookContext) {
	world.commands().entity(cx.entity).observe_any(on_load_run);
}

/// On the entity's [`LoadTemplate`], start its behaviour. A failed build does not
/// run a broken tree; the root's `TemplateError` carries the cause.
fn on_load_run(ev: On<LoadTemplate>, mut commands: Commands) {
	if ev.is_error {
		return;
	}
	commands
		.entity(ev.event_target())
		.call::<(), Outcome>((), default());
}

#[cfg(test)]
mod test {
	use super::*;
	use beet_core::prelude::*;

	#[beet_core::test]
	async fn runs_on_load() {
		let ran = Store::new(false);
		let recorder = ran.clone();
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, TemplatePlugin, ActionPlugin));

		// a tree whose root runs on `LoadTemplate`, recording when its action runs.
		let action: Action<(), Outcome> =
			Action::new_pure(move |_: ActionContext| -> Result<Outcome> {
				recorder.set(true);
				Outcome::PASS.xok()
			});
		app.world_mut()
			.spawn_template(Snippet::from_bundle((RunOnLoad, action)))
			.unwrap();

		// the `LoadTemplate` observer queues the call onto the AsyncWorld; drive the
		// app until it runs.
		for _ in 0..50 {
			app.update();
			if ran.get() {
				break;
			}
			time_ext::sleep_millis(1).await;
		}
		ran.get().xpect_true();
	}
}
