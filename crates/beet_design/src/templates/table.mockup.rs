use crate::prelude::*;
use bevy::prelude::*;



pub fn get() -> impl Bundle {
	let users = vec![
		("Alice", 30, "Engineer"),
		("Bob", 25, "Designer"),
		("Charlie", 35, "Teacher"),
	];


	rsx! {
		 <Table>
			 <tr slot="head">
				 <th>Name</th>
				 <th>Age</th>
				 <th>Occupation</th>
				 <th></th>
			 </tr>
			 <tr>
			 {users.iter().map(|(name, age, occupation)| {
				 rsx! {
					<tr>
					 	<td>{name}</td>
					 	<td>{age}</td>
					 	<td>{occupation}</td>
					 	<td><Button>View Profile</Button></td>
					</tr>
				 }
			 }).collect::<Vec<_>>()}
			 </tr>
		 </Table>
	}
}
