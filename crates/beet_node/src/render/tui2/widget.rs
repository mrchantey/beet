use crate::prelude::*;
use beet_core::prelude::*;

// ── Widget trait ──────────────────────────────────────────────────────────────

pub trait Widget {
	fn layout_style(&self) -> &LayoutStyle;

	/// Pass 1 (bottom-up): given available space as a hint, return desired size.
	fn measure(&self, available: UVec2) -> UVec2;

	/// Pass 2 (top-down): given the assigned rect, emit render cells to buffer.
	fn layout(&self, buffer: &mut Buffer, rect: URect);
}

#[derive(Component, Deref, DerefMut)]
pub struct EntityWidget {
	widget: Box<dyn 'static + Send + Sync + Widget>,
}

impl EntityWidget {
	pub fn new(render: impl 'static + Send + Sync + Widget) -> Self {
		Self {
			widget: Box::new(render),
		}
	}
}

pub struct WidgetRef<'a> {
	entity: Entity,
	widget: &'a EntityWidget,
	children: Vec<WidgetRef<'a>>,
}

impl Widget for WidgetRef<'_> {
	fn layout_style(&self) -> &LayoutStyle { self.widget.layout_style() }
	fn measure(&self, available: UVec2) -> UVec2 {
		self.widget.measure(available)
	}
	fn layout(&self, buffer: &mut Buffer, rect: URect) {
		self.widget.layout(buffer, rect);
	}
}


#[derive(SystemParam)]
pub struct WidgetQuery<'w, 's> {
	// widgets: Query<'w, 's, &'static EntityWidget>,
	changed: Query<'w, 's, Entity, Changed<EntityWidget>>,
	children: Query<'w, 's, &'static Children>,
	widgets: Query<'w, 's, &'static EntityWidget>,
	root_widgets: AncestorQuery<'w, 's, (Entity, &'static EntityWidget)>,
}


impl WidgetQuery<'_, '_> {
	pub fn root_widget(&self, entity: Entity) -> Result<WidgetRef<'_>> {
		let (entity, root_widget) =
			self.root_widgets.find_root_ancestor(entity)?;

		WidgetRef {
			entity,
			widget: root_widget,
			children: self.widget_children(entity),
		}
		.xok()
	}

	fn widget_children(&self, entity: Entity) -> Vec<WidgetRef<'_>> {
		let Ok(children) = self.children.get(entity) else {
			return default();
		};
		children
			.iter()
			.map(|child| {
				if let Ok(widget) = self.widgets.get(child) {
					WidgetRef {
						entity: child,
						widget,
						children: self.widget_children(child),
					}
					.xvec()
				} else {
					// an empty child may have widget children
					self.widget_children(child)
				}
			})
			.flatten()
			.collect()
	}

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


pub(super) fn render_changed(query: WidgetQuery) -> Result {
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
