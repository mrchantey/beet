use crate::prelude::*;
use bevy::prelude::*;




#[derive(Bundle)]
pub struct RlAgentBundle<
	Env: Component + Environment<State = Table::State, Action = Table::Action>,
	Table: Component + QSource,
> {
	pub state: Table::State,
	pub action: Table::Action,
	pub table: Table,
	pub env: Env,
	pub params: QLearnParams,
	pub trainer: Trainer,
}


impl<
		Env: Component + Environment<State = Table::State, Action = Table::Action>,
		Table: Component + QSource,
	> RlAgentBundle<Env, Table>
{
}
