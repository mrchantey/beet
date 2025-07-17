


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



#[cfg(test)]
mod test {
	use bevy::prelude::*;
	use serde::Deserialize;
	use serde::Serialize;
	use sweet::prelude::*;
	use crate::as_beet::*;

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
