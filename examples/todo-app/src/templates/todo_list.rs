#![allow(unused)]
use crate::prelude::*;
use beet::prelude::*;

#[template]
#[derive(Reflect)]
pub fn TodoList() -> impl Bundle {
	let (get_items, set_items) = signal::<Vec<TodoItem>>(vec![TodoItem {
		description: "party".into(),
		created: CrossInstant::now(),
		due: CrossInstant::now(),
	}]);

	let items_table = move || {
		get_items()
			.into_iter()
			.enumerate()
			.map(|(index, item)| {
				OnSpawnClone::insert(move || {
					let remove = move || {
						set_items.update(|prev| {
							prev.remove(index);
						})
					};

					rsx! {<TodoItemView item=item remove=remove/>}
				})
			})
			.collect::<Vec<_>>()
	};

	rsx! {
		 <Table>
			<tr slot="head">
				 <th>Description</th>
				 <th>State</th>
				 <th>Actions</th>
				 <th></th>
			</tr>
			<NewItem create={move|item| set_items.update(move|items|items.push(item))}/>
			{items_table}
		 </Table>
	}
}

#[derive(Clone)]
struct TodoItem {
	description: String,
	created: CrossInstant,
	due: CrossInstant,
}


#[template]
fn TodoItemView(
	item: TodoItem,
	remove: Box<dyn 'static + Send + Sync + Fn()>,
) -> impl Bundle {
	rsx! {
		<tr>
			 <td>{item.description}</td>
			 // <td>{item.created}</td>
			 // <td>{occupation}</td>
				 <td>
					<Button onclick=move||remove()>Remove</Button>
				</td>
		</tr>
	}
}


#[template]
fn NewItem(
	create: Box<dyn 'static + Send + Sync + Fn(TodoItem)>,
) -> impl Bundle {
	let (description, set_description) = signal(String::new());


	let add_item = move || {
		create(TodoItem {
			description: description(),
			created: CrossInstant::now(),
			due: CrossInstant::now(),
		});
		set_description(String::new());
	};

	rsx! {
		<tr>
			<td>
				<TextField
					value={description}
					onchange=move |ev|{set_description(ev.value())}/>
			</td>
			<td>
				<Button onclick=move|| add_item()>Create</Button>
			</td>
		</tr>
	}
}
