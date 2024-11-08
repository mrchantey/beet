use crate::prelude::*;
use bevy::prelude::*;
use std::marker::PhantomData;


pub type InsertOnChange<ChangedComponent, InsertedComponent> =
	OnChange<InsertOnChangeHandler<ChangedComponent, InsertedComponent>>;


#[derive(Reflect)]
pub struct InsertOnChangeHandler<
	ChangedComponent: Component,
	InsertedComponent: Bundle + Clone,
> {
	#[reflect(ignore)]
	phantom: PhantomData<(ChangedComponent, InsertedComponent)>,
}

impl<
		ChangedComponent: Component,
		InsertedComponent: Default + Clone + Bundle + Reflect,
	> OnChangeHandler
	for InsertOnChangeHandler<ChangedComponent, InsertedComponent>
{
	type ChangedComponent = ChangedComponent;
	type Params = InsertedComponent;

	fn handle(
		commands: &mut Commands,
		action_entity: Entity,
		action_component: &OnChange<Self>,
		_changed_entity: Entity,
		_changed_component: &Self::ChangedComponent,
	) {
		action_component.target.insert(
			commands,
			action_entity,
			action_component.params.clone(),
		);
	}
}
