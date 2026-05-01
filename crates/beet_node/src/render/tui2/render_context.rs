use crate::prelude::*;
use beet_core::prelude::*;
// use ratatui::buffer::Buffer;
// use ratatui::prelude::Rect;
// use ratatui::prelude::*;


// #[derive(Resource)]



#[derive(Get)]
pub struct TuiRenderContext<'a> {
	pub entity: EntityWorldMut<'a>,
	/// The full area of the terminal
	pub terminal_area: Rect,
	/// A subset of the terminal area, for the root
	/// this will be the same as the terminal area
	pub draw_area: Rect,
	pub buffer: &'a mut Buffer,
}


#[derive(SystemParam)]
pub struct TuiQuery<'w, 's> {
	// widgets: Query<'w, 's, &'static EntityWidget>,
	changed: Populated<'w, 's, Entity, Changed<EntityWidget>>,
	root_widgets: AncestorQuery<'w, 's, (Entity, &'static EntityWidget)>,
}


impl TuiQuery<'_, '_> {
	pub fn render(&self, entity: Entity) -> Result<Buffer> {
		let size = UVec2::new(80, 24);
		self.render_rect(entity, URect::new(0, 0, size.x, size.y))
	}

	pub fn render_rect(&self, entity: Entity, rect: URect) -> Result<Buffer> {
		let mut buffer = Buffer::new(rect);
		let (_, root_widget) = self.root_widgets.find_root_ancestor(entity)?;
		root_widget.layout(&mut buffer, rect);
		buffer.xok()
	}
}


pub(super) fn render_changed(query: TuiQuery) -> Result {
	let mut root_widgets = HashMap::new();
	for entity in query.changed.iter() {
		let roots = query.root_widgets.get_ancestors(entity);
		let Some((root, root_widget)) = roots.first() else {
			unreachable!("Changed entity must have root");
		};
		root_widgets.insert(*root, *root_widget);
	}

	for (_entity, _widget) in root_widgets {
		todo!("render to some backend resource");
	}

	Ok(())
}
