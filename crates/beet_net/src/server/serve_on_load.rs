//! The [`ServeOnLoad`] load-lifecycle verb: boots a host's declared servers when
//! the entry finishes loading.

use crate::prelude::*;
use beet_core::prelude::*;

/// A load-lifecycle verb that boots the host's declared servers on `LoadTemplate`.
///
/// Declared in a router spread alongside the transports it boots:
///
/// ```bsx
/// <Router {(CliServer, HttpServer, ServeOnLoad)}>
/// ```
///
/// On load it parses the process argv for the `--server` filter (which of several
/// declared servers boot, eg `--server=http`) and the boot config (`--port`, ...),
/// and triggers a [`StartServer`] carrying them. The transports observe
/// `StartServer` in their own `on_add` (servers are transport components, never
/// exchange handlers), so this only coordinates the boot.
///
/// `on_add` registers the `LoadTemplate` observer on the marked entity, so it must
/// sit on the entry root, where `LoadTemplate` fires once the whole subtree is
/// built. On a reload (`LoadTemplate` re-fires) it triggers [`StopServer`] first,
/// tearing down the previously booted transports so none leak.
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component, Default)]
#[component(on_add = on_add)]
pub struct ServeOnLoad;

/// Marks a [`ServeOnLoad`] host that has already booted, so a reload stops the old
/// transports before re-booting. Runtime-only (not reflected), so it never rides a
/// saved scene.
#[derive(Default, Component)]
struct ServersBooted;

/// When present, [`ServeOnLoad`] does not boot on `LoadTemplate`: the tree is
/// being loaded for rendering or inspection (eg `export-static`, `check`) rather
/// than serving, so its declared servers stay dormant.
#[derive(Default, Resource)]
pub struct SuppressServerBoot;

/// Registers the `LoadTemplate` observer on the marked entity, mirroring
/// `CliServer::on_add`.
fn on_add(mut world: DeferredWorld, cx: HookContext) {
	world.commands().entity(cx.entity).observe_any(on_load_template);
}

/// On `LoadTemplate`, boot the host's declared servers from the process argv,
/// stopping any previously booted transports first on a reload.
fn on_load_template(
	ev: On<LoadTemplate>,
	suppressed: Option<Res<SuppressServerBoot>>,
	booted: Query<(), With<ServersBooted>>,
	mut commands: Commands,
) {
	// skip a failed build, or a render/inspect load that must not boot servers.
	if ev.is_error || suppressed.is_some() {
		return;
	}
	let host = ev.event_target();
	// a reload: stop the transports a previous load booted before re-booting.
	if booted.contains(host) {
		commands.entity(host).trigger(StopServer::all);
	}
	commands.entity(host).insert(ServersBooted);

	// parse the process argv: `--server` selects which declared servers boot, the
	// rest of the params flow as boot config. The binary's own `--main` is an extra
	// param the boot ignores, just as the one-shot `CliServer` re-parses argv.
	let params = CliArgs::parse_env().params;
	let server = params.get("server").map(|value| value.to_string());
	commands.entity(host).trigger(move |host| {
		StartServer::from_request(host, server.as_deref(), params)
	});
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// Records the boot/stop events a `ServeOnLoad` triggers, in order.
	#[derive(Default, Resource)]
	struct ServerEvents(Vec<&'static str>);

	fn fire_load(world: &mut World, root: Entity) {
		world
			.entity_mut(root)
			.trigger(|entity| LoadTemplate { entity, is_error: false });
		world.flush();
	}

	/// On `LoadTemplate`, `ServeOnLoad` triggers a `StartServer` to boot the host's
	/// declared servers. The `--server` filter comes from the process argv; the
	/// `from_request` mapping of `--server` to a filter is covered in
	/// `server_events`.
	#[beet_core::test]
	fn boots_declared_servers_on_load() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.init_resource::<ServerEvents>()
			.add_observer(|_: On<StartServer>, mut log: ResMut<ServerEvents>| {
				log.0.push("boot");
			});
		let world = app.world_mut();
		let root = world.spawn(ServeOnLoad).id();
		world.flush();
		fire_load(world, root);
		world.resource::<ServerEvents>().0.xpect_eq(vec!["boot"]);
	}

	/// A reload (`LoadTemplate` re-fires) stops the previously booted transports
	/// before re-booting, so none leak.
	#[beet_core::test]
	fn reload_stops_then_reboots() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.init_resource::<ServerEvents>()
			.add_observer(|_: On<StartServer>, mut log: ResMut<ServerEvents>| {
				log.0.push("boot")
			})
			.add_observer(|_: On<StopServer>, mut log: ResMut<ServerEvents>| {
				log.0.push("stop")
			});
		let world = app.world_mut();
		let root = world.spawn(ServeOnLoad).id();
		world.flush();
		// first load boots; the reload stops the old transports then re-boots.
		fire_load(world, root);
		fire_load(world, root);
		world
			.resource::<ServerEvents>()
			.0
			.xpect_eq(vec!["boot", "stop", "boot"]);
	}
}
