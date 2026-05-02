use crate::prelude::*;
use beet_core::prelude::*;

pub fn render_half(query: &StyledNodeQuery, entity: Entity) -> Result<Buffer> {
	let mut size = terminal_ext::size().unwrap_or_else(|_| UVec2::new(80, 24));
	size.y /= 2;
	render_rect(query, entity, URect::new(0, 0, size.x, size.y))
}
pub fn render(query: &StyledNodeQuery, entity: Entity) -> Result<Buffer> {
	let size = terminal_ext::size().unwrap_or_else(|_| UVec2::new(80, 24));
	render_rect(query, entity, URect::new(0, 0, size.x, size.y))
}

pub fn render_rect(
	query: &StyledNodeQuery,
	entity: Entity,
	rect: URect,
) -> Result<Buffer> {
	let mut buffer = Buffer::new(rect);
	let node = query.get_view(entity);
	layout(&node, &mut buffer, rect)?;
	buffer.xok()
}


pub fn measure() {}
pub fn measure_children() {}
fn layout(node: &StyledNodeView, buffer: &mut Buffer, rect: URect) -> Result {
	TextWidget::layout2(node, buffer, rect)
}


fn layout_element(
	node: &StyledNodeView,
	buffer: &mut Buffer,
	rect: URect,
) -> Result {
	todo!()
}
fn layout_value(
	node: &StyledNodeView,
	buffer: &mut Buffer,
	rect: URect,
) -> Result {
	todo!()
}
