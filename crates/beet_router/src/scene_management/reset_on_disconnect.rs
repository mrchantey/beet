//! [`ResetOnDisconnect`]: a client socket whose closure resets the scene.

use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::sockets::*;

/// Triggers a global [`ResetScene`] whenever this entity's [`Socket`]
/// connection ends ([`SocketClosed`]), so hardware returns to its resting state
/// the moment its controller disappears (eg a robot halting its motors when the
/// agent socket drops), not only when the scene is swapped.
///
/// Add it beside a client socket, eg a [`PersistentSocket`]. An established
/// connection fires exactly one [`SocketClosed`] when it ends, and failed
/// redial attempts fire none, so a reconnect loop resets once per drop rather
/// than once per attempt.
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component, Default)]
#[component(on_add = on_add)]
pub struct ResetOnDisconnect;

fn on_add(mut world: DeferredWorld, cx: HookContext) {
	// attach the reset observer through the entity command (like the socket
	// servers' own `observe_any` wiring) rather than hand-spawning an
	// `Observer::new(..).with_entity(..)`. `observe_any`, not `observe`, since
	// [`SocketClosed`] is an [`EntityTargetEvent`], not a Bevy `EntityEvent`.
	world.commands().entity(cx.entity).observe_any(
		|_ev: On<SocketClosed>, mut commands: Commands| {
			commands.trigger(ResetScene);
		},
	);
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::exports::futures_lite;
	use beet_core::prelude::*;
	use beet_net::sockets::*;
	// disambiguate the socket `Message` enum from bevy's `Message` trait (both
	// glob-imported above)
	use beet_net::sockets::Message;

	/// A writer whose sends vanish; only closure matters here.
	struct NoopWriter;
	impl SocketWriter for NoopWriter {
		fn send_boxed(&mut self, _msg: Message) -> SendBoxedFuture<Result<()>> {
			Box::pin(async { Ok(()) })
		}
		fn close_boxed(
			&mut self,
			_close: Option<CloseFrame>,
		) -> SendBoxedFuture<Result<()>> {
			Box::pin(async { Ok(()) })
		}
	}

	/// A socket whose transport drops immediately fires [`SocketClosed`], which
	/// [`ResetOnDisconnect`] turns into a global [`ResetScene`].
	#[beet_core::test]
	async fn resets_on_socket_closed() {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, AsyncPlugin));
		let resets = Store::<Vec<()>>::default();
		app.add_observer(move |_ev: On<ResetScene>| {
			resets.push(());
		});
		app.world_mut().spawn((
			ResetOnDisconnect,
			Socket::new(
				futures_lite::stream::once(Err::<Message, BevyError>(bevyhow!(
					"peer dropped"
				))),
				NoopWriter,
			),
		));
		app_ext::update_until(&mut app, move |_| !resets.is_empty())
			.await
			.xpect_true();
	}
}
