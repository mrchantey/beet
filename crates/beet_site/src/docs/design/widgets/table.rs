use beet::prelude::*;

/// Shows the [`Table`] widget: the default horizontal-rule layout and the
/// `vertical_lines` grid variant, sharing one set of demo rows.
pub fn get() -> impl Scene {
	rsx! {
		<article>
			<h1>"Table"</h1>
			<h2>"Default"</h2>
			<Table>
				<tr slot="head">{head_cells()}</tr>
				{body_rows()}
			</Table>
			<h2>"Vertical lines"</h2>
			<Table vertical_lines=true>
				<tr slot="head">{head_cells()}</tr>
				{body_rows()}
			</Table>
		</article>
	}
}

/// The shared header row cells.
fn head_cells() -> Vec<Box<dyn Scene>> {
	vec![
		rsx! { <th>"Name"</th> }.any_scene(),
		rsx! { <th>"Age"</th> }.any_scene(),
		rsx! { <th>"Occupation"</th> }.any_scene(),
		rsx! { <th></th> }.any_scene(),
	]
}

/// The shared demo body rows.
fn body_rows() -> Vec<Box<dyn Scene>> {
	let users = [
		("Alice", 30, "Engineer"),
		("Bob", 25, "Designer"),
		("Charlie", 35, "Teacher"),
	];
	users
		.iter()
		.map(|(name, age, occupation)| {
			rsx! {
				<tr>
					<td>{name}</td>
					<td>{age}</td>
					<td>{occupation}</td>
					<td>
						<Button variant=ButtonVariant::Text>"View Profile"</Button>
					</td>
				</tr>
			}
			.any_scene()
		})
		.collect()
}
