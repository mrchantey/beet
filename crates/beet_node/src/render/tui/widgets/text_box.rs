use std::borrow::Cow;

use crate::prelude::*;
use beet_core::prelude::*;
use ratatui::prelude::*;
use ratatui::widgets::Block;
use ratatui::widgets::Paragraph;

#[derive(Clone, Component)]
#[require(TuiWidget=widget())]
pub struct TuiTextBox {
	label: Cow<'static, str>,
	value: String,
}

impl TuiTextBox {
	pub fn new(
		label: impl Into<Cow<'static, str>>,
		value: impl ToString,
	) -> Self {
		Self {
			label: label.into(),
			value: value.to_string(),
		}
	}
}

fn widget() -> TuiWidget {
	TuiWidget::new(Constraint::Length(3), |cx| {
		let TuiTextBox { label, value } = cx.entity.get().ok_or_else(|| {
			bevyhow!(
				"TuiTextBox component missing from entity {:?}",
				cx.entity.id()
			)
		})?;
		let block = Block::bordered().title(label.to_string());
		let inner_area = block.inner(cx.draw_area);
		block.render(cx.draw_area, cx.buffer);

		let paragraph = Paragraph::new(value.as_str());
		paragraph.render(inner_area, cx.buffer);
		Ok(())
	})
}
