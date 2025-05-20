use crate::prelude::*;
use beet_common::prelude::*;
use bevy::prelude::*;
use proc_macro2::TokenStream;
use proc_macro2_diagnostics::Diagnostic;
use quote::quote;
use rstml::Parser;
use rstml::ParserConfig;
use rstml::node::Node;
use sweet::prelude::WorkspacePathBuf;

// we must use `std::collections::HashSet` because thats what rstml uses
type HashSet<T> = std::collections::HashSet<T>;
/// definition for the rstml custom node, currently unused
pub(super) type RstmlCustomNode = rstml::Infallible;

pub fn tokens_to_rstml_plugin(app: &mut App) {
	app.init_resource::<RstmlConfig>()
		.init_non_send_resource::<NonSendAssets<RstmlTokens>>()
		.init_non_send_resource::<NonSendAssets<RstmlNodes>>()
		.init_non_send_resource::<NonSendAssets<TokensDiagnostics>>()
		.add_systems(Update, tokens_to_rstml.in_set(ImportNodesStep));
	let rstml_config = app.world().resource::<RstmlConfig>();
	app.insert_non_send_resource(parser_config(rstml_config));
}

fn parser_config(rstml_config: &RstmlConfig) -> Parser<RstmlCustomNode> {
	let config = ParserConfig::new()
		.recover_block(true)
		.always_self_closed_elements(rstml_config.self_closing_elements.clone())
		.raw_text_elements(rstml_config.raw_text_elements.clone())
		// here we define the rsx! macro as the constant thats used
		// to resolve raw text blocks more correctly
		.macro_call_pattern(quote!(rsx! {%%}))
		.custom_node::<RstmlCustomNode>();
	Parser::new(config)
}

/// Hashset of element tag names that should be self-closing.
#[derive(Resource)]
pub struct RstmlConfig {
	pub raw_text_elements: HashSet<&'static str>,
	pub self_closing_elements: HashSet<&'static str>,
}

impl Default for RstmlConfig {
	fn default() -> Self {
		Self {
			raw_text_elements: ["script", "style"].into_iter().collect(),
			self_closing_elements: [
				"area", "base", "br", "col", "embed", "hr", "img", "input",
				"link", "meta", "param", "source", "track", "wbr",
			]
			.into_iter()
			.collect(),
		}
	}
}

/// A [`WorkspacePathBuf`] representing the source file for the contents of
/// this entity.
// When we get Construct this should be a pointer to reduce needless string allocation
#[derive(Debug, Clone, Component, Deref)]
pub struct SourceFile(WorkspacePathBuf);

impl SourceFile {
	pub fn new(path: WorkspacePathBuf) -> Self { Self(path) }
}

/// A [`TokenStream`] representing [`rstml`] flavored rsx tokens.
#[derive(Debug, Clone, Deref)]
pub struct RstmlTokens(TokenStream);
impl RstmlTokens {
	pub fn new(tokens: TokenStream) -> Self { Self(tokens) }
	pub fn into_inner(self) -> TokenStream { self.0 }
}


/// A vec of [`rstml::node::Node`] retrieved from the [`RstmlTokens`]
/// via [`tokens_to_rstml`].
#[derive(Debug, Clone, Deref)]
pub struct RstmlNodes(Vec<Node>);
impl RstmlNodes {
	pub fn new(nodes: Vec<Node>) -> Self { Self(nodes) }
	pub fn into_inner(self) -> Vec<Node> { self.0 }
}

#[derive(Debug, Deref, DerefMut)]
pub struct TokensDiagnostics(Vec<Diagnostic>);

impl TokensDiagnostics {
	pub fn new(value: Vec<Diagnostic>) -> Self { Self(value) }
	pub fn into_inner(self) -> Vec<Diagnostic> { self.0 }
	pub fn into_tokens(self) -> Vec<TokenStream> {
		self.0.into_iter().map(|d| d.emit_as_expr_tokens()).collect()
	}
}


/// Replace the tokens
pub(super) fn tokens_to_rstml(
	mut commands: Commands,
	parser: NonSend<Parser<RstmlCustomNode>>,
	mut tokens_map: NonSendMut<NonSendAssets<RstmlTokens>>,
	mut diagnostics_map: NonSendMut<NonSendAssets<TokensDiagnostics>>,
	mut nodes_map: NonSendMut<NonSendAssets<RstmlNodes>>,
	query: Populated<(Entity, &NonSendHandle<RstmlTokens>)>,
) -> Result {
	for (entity, handle) in query.iter() {
		let rstml_tokens = tokens_map.remove(handle)?;
		let (nodes, errors) = parser
			.parse_recoverable(rstml_tokens.into_inner())
			.split_vec();
		commands
			.entity(entity)
			.remove::<NonSendHandle<RstmlTokens>>()
			.insert((
				nodes_map.insert(RstmlNodes::new(nodes)),
				diagnostics_map.insert(TokensDiagnostics::new(errors)),
			));
	}
	Ok(())
}



#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_common::prelude::EntityWorldMutInsertNonSendExt;
	use beet_common::prelude::NonSendAssets;
	use beet_common::prelude::NonSendHandle;
	use bevy::ecs::system::RunSystemOnce;
	use bevy::prelude::*;
	use proc_macro2::TokenStream;
	use quote::quote;
	use sweet::prelude::*;


	fn parse(tokens: TokenStream) -> Vec<RstmlNodes> {
		App::new()
			.add_plugins(tokens_to_rstml_plugin)
			.xtap(|app| {
				app.world_mut()
					.spawn_empty()
					.insert_non_send(RstmlTokens::new(tokens));
			})
			.update_then()
			.world_mut()
			.run_system_once(
				|mut nodes: NonSendMut<NonSendAssets<RstmlNodes>>,
				 query: Query<&NonSendHandle<RstmlNodes>>| {
					query
						.iter()
						.map(move |handle| nodes.remove(handle).unwrap())
						.collect()
				},
			)
			.unwrap()
	}


	#[test]
	fn works() {
		quote! {
			<MyComponent client:load />
			<div/>
		}
		.xmap(parse)
		.xmap(|nodes| nodes[0].len())
		.xpect()
		.to_be(2);
	}
}
