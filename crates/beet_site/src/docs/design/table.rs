use beet::prelude::*;

/// Shows the [`Table`] widget: the default horizontal-rule layout and the
/// `vertical_lines` grid variant, sharing one set of demo rows.
pub fn get() -> impl Bundle {
	rsx! {
		<article>
			<h1>"Table"</h1>
			<h2>"Default"</h2>
			<Table>
				<tr slot="head">
					<th>"Name"</th>
					<th>"Age"</th>
					<th>"Occupation"</th>
					<th></th>
				</tr>
				{user_row("Alice", 30, "Engineer")}
				{user_row("Bob", 25, "Designer")}
				{user_row("Charlie", 35, "Teacher")}
			</Table>
			<h2>"Vertical lines"</h2>
			<Table vertical_lines=true>
				<tr slot="head">
					<th>"Name"</th>
					<th>"Age"</th>
					<th>"Occupation"</th>
					<th></th>
				</tr>
				{user_row("Alice", 30, "Engineer")}
				{user_row("Bob", 25, "Designer")}
				{user_row("Charlie", 35, "Teacher")}
			</Table>
		</article>
	}
}

/// A demo body row: the user's details and a profile action.
fn user_row(name: &str, age: u32, occupation: &str) -> impl Bundle {
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
}
