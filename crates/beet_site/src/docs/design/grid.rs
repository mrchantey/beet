use beet::prelude::*;

/// The grid display mode: row-major flow into equal-width column tracks.
///
/// The `.grid` class is the conventional 12 columns of square tracks (on the
/// terminal a square is half the column width, cells being ~2:1) with a
/// one-cell gap; `grid-template-columns` / `grid-auto-rows` rules adjust both.
pub fn get() -> impl Bundle {
	rsx! {
		<article>
			<h1>"Grid"</h1>
			<h2>"Default: 12 columns, square tracks"</h2>
			<div {Classes::new([classes::GRID])}>
				{(1..=12).map(grid_cell).collect::<Vec<_>>()}
			</div>
			<h2>"4 columns"</h2>
			<div
				{Classes::new([classes::GRID])}
				{inline_class![(common_props::GridTemplateColumnsProp, GridColumns(4))]}
			>
				{(1..=8).map(grid_cell).collect::<Vec<_>>()}
			</div>
			<h2>"3 columns, 2-row tracks"</h2>
			<div
				{Classes::new([classes::GRID])}
				{inline_class![
					(common_props::GridTemplateColumnsProp, GridColumns(3)),
					(common_props::GridAutoRowsProp, GridRows::Length(Length::Rem(2.))),
				]}
			>
				{(1..=6).map(grid_cell).collect::<Vec<_>>()}
			</div>
		</article>
	}
}

/// A numbered, filled cell so each track reads at a glance.
fn grid_cell(index: usize) -> impl Bundle {
	rsx! {
		<div {Classes::new([classes::CARD_FILLED])}>{index.to_string()}</div>
	}
}
