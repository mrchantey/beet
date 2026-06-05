use beet::prelude::*;

/// Shows the [`Table`] widget with head and body rows.
pub fn get() -> impl Scene {
	let users = [
		("Alice", 30, "Engineer"),
		("Bob", 25, "Designer"),
		("Charlie", 35, "Teacher"),
	];
	rsx! {
		<article>
			<h1>"Table"</h1>
			<Table>
				<tr slot="head">
					<th>"Name"</th>
					<th>"Age"</th>
					<th>"Occupation"</th>
					<th></th>
				</tr>
				{users
					.iter()
					.map(|(name, age, occupation)| {
						rsx! {
							<tr>
								<td>{name}</td>
								<td>{age}</td>
								<td>{occupation}</td>
								<td>
									<Button label="View Profile" variant=ButtonVariant::Text/>
								</td>
							</tr>
						}
					})
					.collect::<Vec<_>>()}
			</Table>
		</article>
	}
}
