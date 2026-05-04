use crate::prelude::*;
use crate::style::*;
use beet_core::prelude::*;

#[derive(Clone)]
pub struct StyledNodeView<'a> {
	pub entity: Entity,
	pub element: Option<ElementView<'a>>,
	pub value: Option<&'a Value>,
	pub flexbox: Option<&'a FlexBox>,
	pub visual: Option<&'a VisualStyle>,
	pub layout: Option<&'a LayoutStyle>,
	pub children: Vec<StyledNodeView<'a>>,
}


impl<'a> StyledNodeView<'a> {
	pub fn visual_style(&self) -> &VisualStyle {
		self.visual.unwrap_or(&VISUAL_STYLE_DEFAULT)
	}
	pub fn layout_style(&self) -> &LayoutStyle {
		self.layout.unwrap_or(&LAYOUT_STYLE_DEFAULT)
	}
}

#[derive(SystemParam)]
pub struct StyledNodeQuery<'w, 's> {
	elements: ElementQuery<'w, 's>,
	styled_nodes: Query<
		'w,
		's,
		(
			Option<&'static Value>,
			Option<&'static FlexBox>,
			Option<&'static VisualStyle>,
			Option<&'static LayoutStyle>,
		),
	>,
	children: Query<'w, 's, &'static Children>,
}


impl<'w, 's> StyledNodeQuery<'w, 's> {
	/// Create a [`StyledNodeView`] for the provided entity,
	/// recursively creating for children as well.
	pub fn get_view(&self, entity: Entity) -> StyledNodeView<'_> {
		let element = self.elements.get(entity).ok();
		let (value, flexbox, visual, layout) =
			self.styled_nodes.get(entity).unwrap_or_default();
		let children = self
			.children
			.get(entity)
			.map(|c| c.iter().map(|e| self.get_view(e)).collect())
			.unwrap_or_default();

		StyledNodeView {
			entity,
			element,
			value,
			flexbox,
			visual,
			layout,
			children,
		}
	}
}
