use bevy::prelude::*;



#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
pub struct ApplySlotsStep;

/// Slotting is the process of traversing the [RsxComponent::slot_children]
/// and applying them to the [RsxComponent::node] in the corresponding slots.
///
/// ```ignore
/// # use beet_rsx::as_beet::*;
///
/// #[derive(Node)]
/// struct MyComponent;
///
///
/// fn my_component(_: MyComponent)->WebNode {
/// 	rsx!{
/// 		<html>
/// 			<slot name="header"/>
/// 			<slot/> //default
/// 		</html>
/// 	}
/// }
///
/// assert_eq!(rsx!{
/// 	<MyComponent>
///  		<div slot="header">Header</div>
/// 		<div>Default</div>
///  	</MyComponent>
/// }.xpipe(RsxToHtmlString::default()).unwrap(),
/// "<html><div>Header</div><div>Default</div></html>");
///
/// ```
///
/// ## Slot Rules
///
/// - Slot children will be inserted into the first slot with a matching name,
/// 	found via [RsxVisitor] dfs traversal.
/// - Only top level slots, fragments aside, are supported
/// - Any unconsumed slot children will be returned in an error
/// - For unnamed slots `<div/>`, they will be inserted in the components unnamed
/// 	<sag.
/// - Comnot recursively searched because they would steal the slot
/// 	from following internal siblings.
/// - All <slot> elements are replaced with a <fragment> element containing the
/// 	slot children.
/// - All slot="foo" attributes are removed.
pub fn apply_slots_plugin(app: &mut App) {}


fn apply_slots(// query:Populated<
) {
}
