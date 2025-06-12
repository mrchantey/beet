use crate::prelude::*;
use beet_common::prelude::*;
use beet_utils::prelude::*;
use bevy::prelude::*;
use proc_macro2::TokenStream;
use proc_macro2_diagnostics::Diagnostic;
use quote::quote;
use rstml::Parser;
use rstml::ParserConfig;
use rstml::node::Node;
use send_wrapper::SendWrapper;

// we must use `std::collections::HashSet` because thats what rstml uses
type HashSet<T> = std::collections::HashSet<T>;
/// definition for the rstml custom node, currently unused
pub(super) type RstmlCustomNode = rstml::Infallible;

pub fn tokens_to_rstml_plugin(app: &mut App) {
	app.init_resource::<RstmlConfig>()
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
#[derive(Debug, Clone, Deref, Component)]
pub struct RstmlTokens(SendWrapper<TokenStream>);
impl RstmlTokens {
	pub fn new(tokens: TokenStream) -> Self { Self(SendWrapper::new(tokens)) }
	pub fn take(self) -> TokenStream { self.0.take() }
}


/// A vec of [`rstml::node::Node`] retrieved from the [`RstmlTokens`]
/// via [`tokens_to_rstml`].
#[derive(Debug, Clone, Deref, DerefMut, Component)]
pub struct RstmlNodes(SendWrapper<Vec<Node>>);
impl RstmlNodes {
	pub fn new(nodes: Vec<Node>) -> Self { Self(SendWrapper::new(nodes)) }
	pub fn take(self) -> Vec<Node> { self.0.take() }
}

#[derive(Debug, Deref, DerefMut, Component)]
pub struct TokensDiagnostics(pub SendWrapper<Vec<Diagnostic>>);

impl TokensDiagnostics {
	pub fn new(value: Vec<Diagnostic>) -> Self { Self(SendWrapper::new(value)) }
	pub fn take(self) -> Vec<Diagnostic> { self.0.take() }
	pub fn into_tokens(self) -> Vec<TokenStream> {
		self.take()
			.into_iter()
			.map(|d| d.emit_as_expr_tokens())
			.collect()
	}
}


/// Replace the tokens
pub(super) fn tokens_to_rstml(
	_: TempNonSendMarker,
	mut commands: Commands,
	parser: NonSend<Parser<RstmlCustomNode>>,
	query: Populated<(Entity, &RstmlTokens)>,
) -> Result {
	for (entity, handle) in query.iter() {
		let tokens = handle.clone().take();
		let (nodes, errors) = parser.parse_recoverable(tokens).split_vec();
		commands
			.entity(entity)
			.remove::<RstmlTokens>()
			.insert((RstmlNodes::new(nodes), TokensDiagnostics::new(errors)));
	}
	Ok(())
}



#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_bevy::prelude::*;
	use beet_utils::prelude::*;
	use bevy::prelude::*;
	use proc_macro2::TokenStream;
	use quote::quote;
	use sweet::prelude::*;


	fn parse(tokens: TokenStream) -> Vec<RstmlNodes> {
		App::new()
			.add_plugins(tokens_to_rstml_plugin)
			.xtap(|app| {
				app.world_mut().spawn(RstmlTokens::new(tokens));
			})
			.update_then()
			.remove::<RstmlNodes>()
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
