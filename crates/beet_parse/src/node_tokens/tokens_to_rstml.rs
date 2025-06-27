use crate::prelude::*;
use beet_common::prelude::*;
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

/// A [`TokenStream`] representing [`rstml`] flavored rsx tokens.
#[derive(Debug, Clone, Deref, Component)]
#[require(MacroIdx)]
pub struct RstmlTokens(SendWrapper<TokenStream>);
impl RstmlTokens {
	pub fn new(tokens: TokenStream) -> Self { Self(SendWrapper::new(tokens)) }
	pub fn take(self) -> TokenStream { self.0.take() }
}


/// A vec of [`rstml::node::Node`] retrieved from the [`RstmlTokens`]
/// via [`tokens_to_rstml`].
#[derive(Debug, Clone, Deref, DerefMut, Component)]
#[require(MacroIdx)]
pub struct RstmlRoot(SendWrapper<Node>);
impl RstmlRoot {
	pub fn new(node: Node) -> Self { Self(SendWrapper::new(node)) }
	pub fn take(self) -> Node { self.0.take() }
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


/// Replace the tokens with parsed [`RstmlNodes`], and apply a [`MacroIdx`]
pub(super) fn tokens_to_rstml(
	_: TempNonSendMarker,
	mut commands: Commands,
	parser: NonSend<Parser<RstmlCustomNode>>,
	query: Populated<(Entity, &RstmlTokens), Added<RstmlTokens>>,
) -> Result {
	for (entity, handle) in query.iter() {
		let tokens = handle.clone().take();
		// this is the key to matching statically analyzed macros
		// with instantiated ones
		let (nodes, errors) = parser.parse_recoverable(tokens).split_vec();

		let node = match nodes.len() {
			0 => rstml_fragment(vec![]),
			1 => nodes.into_iter().next().unwrap(),
			_ => rstml_fragment(nodes),
		};

		commands
			.entity(entity)
			.remove::<RstmlTokens>()
			.insert((RstmlRoot::new(node), TokensDiagnostics::new(errors)));
	}
	Ok(())
}

/// create an rstml fragment node
fn rstml_fragment(
	children: Vec<Node<RstmlCustomNode>>,
) -> Node<RstmlCustomNode> {
	// i guess we dont attempt to create a span, thats untruthful
	// 	let span = if nodes.len() == 1 {
	// 	nodes.first().unwrap().span()
	// } else {
	// 	nodes
	// 		.first()
	// 		.map(|n| n.span())
	// 		.unwrap_or(Span::call_site())
	// 		.join(
	// 			nodes.last().map(|n| n.span()).unwrap_or(Span::call_site()),
	// 		)
	// 		.unwrap_or(Span::call_site())
	// };

	Node::Fragment(rstml::node::NodeFragment {
		tag_open: rstml::node::atoms::FragmentOpen {
			token_lt: Default::default(),
			token_gt: Default::default(),
		},
		children,
		tag_close: Some(rstml::node::atoms::FragmentClose {
			start_tag: rstml::node::atoms::CloseTagStart {
				token_lt: Default::default(),
				token_solidus: Default::default(),
			},
			token_gt: Default::default(),
		}),
	})
}

/// see rstml_to_node_tokens.rs for more tests
#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_bevy::prelude::*;
	use beet_utils::prelude::*;
	use bevy::prelude::*;
	use proc_macro2::TokenStream;
	use quote::quote;
	use rstml::node::Node;
	use sweet::prelude::*;


	fn parse(tokens: TokenStream) -> Vec<RstmlRoot> {
		App::new()
			.add_plugins(tokens_to_rstml_plugin)
			.xtap(|app| {
				app.world_mut().spawn(RstmlTokens::new(tokens));
			})
			.update_then()
			.remove::<RstmlRoot>()
	}


	#[test]
	fn works() {
		quote! {
			<MyComponent client:load />
			<div/>
		}
		.xmap(parse)
		.xmap(|nodes| {
			let mut nodes = nodes;
			let Node::Fragment(fragment) =
				nodes.first_mut().unwrap().clone().take()
			else {
				panic!();
			};
			fragment.children().len()
		})
		.xpect()
		.to_be(2);
	}
}
