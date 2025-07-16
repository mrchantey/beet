use beet_core::prelude::*;
use bevy::ecs::component::Immutable;
use bevy::ecs::component::StorageType;
use bevy::prelude::*;
use serde::Deserialize;
use serde::Serialize;




#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientIsland {
	pub template: TemplateSerde,
	/// Whether to create html elements instead of hydrating them,
	/// if the template is [`ClientOnlyDirective`], this will be true.
	pub mount_to_dom: bool,
	pub dom_idx: DomIdx, // pub route: RouteInfo,
}

pub fn collect_client_islands(
	In(root): In<Entity>,
	children: Query<&Children>,
	islands: Query<(&TemplateSerde, &DomIdx, Option<&ClientOnlyDirective>)>,
) -> Vec<ClientIsland> {
	children
		.iter_descendants_inclusive(root)
		.filter_map(|entity| {
			islands
				.get(entity)
				.ok()
				.map(|(template, dom_idx, client_only)| {
					let mount = client_only.is_some();
					ClientIsland {
						template: template.clone(),
						dom_idx: *dom_idx,
						mount_to_dom: mount,
						// route: RouteInfo::from_tracker(tracker),
					}
				})
		})
		.collect()
}

#[derive(Debug, Clone, Reflect)]
#[reflect(Component)]
pub struct SpawnClientIsland<T, U = T>
where
	T: 'static + Send + Sync + IntoTemplateBundle<U>,
	U: 'static + Send + Sync,
{
	value: T,
	#[reflect(ignore)]
	phantom: std::marker::PhantomData<U>,
}


impl<T, U> Component for SpawnClientIsland<T, U>
where
	T: 'static + Send + Sync + IntoTemplateBundle<U>,
	U: 'static + Send + Sync,
{
	const STORAGE_TYPE: StorageType = StorageType::Table;
	type Mutability = Immutable;


	fn on_add() -> Option<bevy::ecs::component::ComponentHook> {
		Some(|mut world, cx| {
			let entity = cx.entity;
			world.commands().queue(move |world: &mut World| -> Result {
				let this = world.entity_mut(entity).take::<Self>().ok_or_else(
					|| {
						let name = std::any::type_name::<Self>();
						bevyhow!(
							"SpawnClientIsland<{}> component not found on entity: {:?}",
							name,
							entity
						)
					},
				)?;
				world.entity_mut(entity).insert(this.value.into_node_bundle());
				Ok(())
			});
		})
	}
}


#[cfg(test)]
#[cfg(not(target_arch = "wasm32"))]
mod test {
	use crate::prelude::*;
	use bevy::prelude::*;
	use serde::Deserialize;
	use serde::Serialize;
	use sweet::prelude::*;

	use beet_rsx::as_beet::*;

	#[template]
	#[derive(Serialize, Deserialize)]
	pub fn MyTemplate(foo: u32) -> impl Bundle {
		let _ = foo;
		()
	}

	fn collect(
		app: &mut App,
		bundle: impl Bundle,
	) -> Result<Vec<ClientIsland>> {
		let entity = app.world_mut().spawn((HtmlDocument, bundle)).id();
		app.update();
		app.world_mut()
			.run_system_cached_with(collect_client_islands, entity)?
			.xok()
	}

	#[test]
	fn works() {
		let mut app = App::new();
		app.add_plugins(TemplatePlugin::default());
		app.insert_resource(TemplateFlags::None);

		collect(&mut app, rsx! {
			<MyTemplate foo=3 client:only />
		})
		.unwrap()
		.xpect()
		.to_be(vec![ClientIsland {
			template: TemplateSerde::new(&MyTemplate { foo: 3 }),
			dom_idx: DomIdx::new(0),
			mount_to_dom: true,
		}]);
	}
}
