use beet_ecs::ecs_nodes::EcsNode;
use strum::IntoEnumIterator;
use sweet::*;

#[sweet_test]
pub fn works() -> Result<()> {
	expect(EcsNode::iter().count()).to_be_greater_than(0)?;

	Ok(())
}
