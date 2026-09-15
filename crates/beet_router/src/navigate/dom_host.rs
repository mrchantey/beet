//! [`DomHost`]: the browser surface, the page bound to it painted into the
//! document body once the page settles.

use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_ui::prelude::*;

/// The browser surface: the page bound to it is what the document shows.
///
/// A [`PageHost`] with its [`PageSlot`] and no buffer of its own, since its
/// paint target is the document body rather than a cell grid. Spawned as the
/// [`DomServer`]'s child so it goes with it, carrying the in-world
/// [`Navigator`] that binds the page.
///
/// The first paint waits for the page to settle ([`Self::settled`]): the
/// served page stays visible and inert until the world matches it, then the
/// world adopts the body's children where they agree and replaces them where
/// they do not ([`DomRenderer::mount`]), reveals what a returning editor's
/// page hid ([`PreBoot::reveal`]) and the incremental pass keeps them
/// following it. Painted means bound: the host carries the [`DomNode`] of the
/// body it painted into, so a change under it reconciles against the body.
/// The paint's log line carries its [`Adoption`], the conformance measure a
/// browser suite reads: a page served by the same entry adopts with nothing
/// replaced.
#[derive(Debug, Default, Clone, Component)]
#[require(PageHost)]
#[component(on_add = hook_ext::observe(log_landing))]
pub struct DomHost;

impl DomHost {
	/// The host with its page slot, ready for a co-located [`Navigator`].
	pub fn bundle() -> impl Bundle { (DomHost, children![PageSlot]) }

	/// Whether nothing under `host` is still arriving: no store reader marked
	/// [`Loading`] and no template dependency pending, crossing each
	/// [`Portal`] as the render walk does.
	pub fn settled(world: &World, host: Entity) -> bool {
		let mut stack = vec![host];
		while let Some(entity) = stack.pop() {
			let Ok(entity) = world.get_entity(entity) else {
				continue;
			};
			if entity.contains::<Loading>()
				|| entity
					.get::<TemplatePending>()
					.is_some_and(|pending| !pending.is_empty())
			{
				return false;
			}
			stack.extend(
				entity
					.get::<Children>()
					.into_iter()
					.flat_map(|children| children.iter()),
			);
			stack.extend(entity.get::<Portal>().map(Portal::target));
		}
		true
	}
}

/// Observer: the host's page landed, so the tab's console says which url the
/// world dispatched. The boot check reads this line, then the paint's.
fn log_landing(ev: On<Insert, RenderSurfaceOf>, navigators: Query<&Navigator>) {
	if let Ok(navigator) = navigators.get(ev.entity) {
		info!("dom host landed {}", navigator.current_url());
	}
}

/// System: paint each host whose bound page has settled, once, into the
/// document body.
#[cfg(target_arch = "wasm32")]
pub(crate) fn mount_dom_hosts(world: &mut World) {
	let hosts: Vec<Entity> = world
		.query_filtered::<Entity, (
			With<DomHost>,
			With<RenderSurfaceOf>,
			Without<DomNode>,
		)>()
		.iter(world)
		.collect();
	for host in hosts {
		if !DomHost::settled(world, host) {
			continue;
		}
		let adoption =
			DomRenderer::mount(world, host, document_ext::body().into());
		PreBoot::reveal();
		let url = world
			.get::<Navigator>(host)
			.map(|navigator| navigator.current_url().to_string())
			.unwrap_or_default();
		info!("dom host painted {url} ({adoption})");
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;
	use beet_ui::prelude::*;

	/// A host paints once nothing under its page is still arriving: a store
	/// reader marked loading, or a template dependency pending, holds the
	/// first paint, across the slot's transclusion.
	#[beet_core::test]
	fn settles_once_nothing_under_the_page_loads() {
		let mut world = World::new();
		let page = world
			.spawn(children![(Element::new("main"), Loading::Pending)])
			.id();
		let host = world
			.spawn((DomHost, children![(PageSlot, Portal::new(page))]))
			.id();
		DomHost::settled(&world, host).xpect_false();
		let reader = world.get::<Children>(page).unwrap()[0];
		world.entity_mut(reader).remove::<Loading>();
		DomHost::settled(&world, host).xpect_true();
		// a pending template dependency holds it too
		let mut pending = TemplatePending::default();
		pending.register(PendingKind::Structural, "<Template src>");
		world.entity_mut(page).insert(pending);
		DomHost::settled(&world, host).xpect_false();
	}
}
