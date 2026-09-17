//! The vault declarations: a file and who may read it.

use crate::prelude::*;

/// A vault: one age-encrypted file holding a document in one of the
/// [`VaultFormat`]s, at a path in a store, encrypted to a recipient list.
///
/// ```bsx
/// <Vault label=".env" path=".env.age" recipients={["age1..", "age1.."]}/>
/// <Vault label="mail-cold" path="secrets" {StoreRef($cold_backups)}/>
/// ```
///
/// The store is the `StoreRef` target's when the declaration carries one,
/// else the nearest ancestor `BlobStore` (the repo store), so a committed
/// vault is a plain relative path. The recipients are the vault's own, else
/// the nearest ancestor [`AgeRecipients`]: age files do not reveal their
/// recipients, so the declaration is the source of truth for who can read
/// the next write. Decrypting needs no declaration, only an identity.
///
/// Data, so it registers in every std build and a lean binary loads a
/// document naming one; the verbs that open it ride the `secrets` feature.
#[derive(Debug, Clone, PartialEq, Eq, Component, Reflect)]
#[reflect(Component, Default)]
pub struct Vault {
	/// How the vault verbs name it (`--vault=<label>`); `.env` is the default
	/// vault every verb takes when none is named.
	pub label: SmolStr,
	/// The file, relative to its store; the format is its extension before
	/// `.age`.
	pub path: RelPath,
	/// Who may read the next write; empty inherits an ancestor's
	/// [`AgeRecipients`].
	pub recipients: Vec<AgeRecipient>,
}

impl Default for Vault {
	/// The entry's `.env.age`, the vault every verb defaults to.
	fn default() -> Self {
		Self {
			label: SmolStr::new_static(Self::ENV_LABEL),
			path: RelPath::new(Self::ENV_PATH),
			recipients: Vec::new(),
		}
	}
}

impl Vault {
	/// The default vault's label: the entry's `.env.age`.
	pub const ENV_LABEL: &'static str = ".env";
	/// The default vault's path, beside the entry.
	pub const ENV_PATH: &'static str = ".env.age";

	/// A vault at `path`, labelled by its file name.
	pub fn new(path: impl AsRef<str>) -> Self {
		let path = RelPath::new(path);
		Self {
			label: SmolStr::new(path.file_name().unwrap_or_default()),
			path,
			recipients: Vec::new(),
		}
	}

	/// Set the label.
	pub fn with_label(mut self, label: impl Into<SmolStr>) -> Self {
		self.label = label.into();
		self
	}

	/// Set the recipients.
	pub fn with_recipients(
		mut self,
		recipients: impl IntoIterator<Item = AgeRecipient>,
	) -> Self {
		self.recipients = recipients.into_iter().collect();
		self
	}

	/// The format the path names.
	pub fn format(&self) -> Result<VaultFormat> {
		VaultFormat::from_path(self.path.as_str())
	}
}

/// The recipient list every [`Vault`] beneath inherits, authored on a
/// `<Stack>` or any ancestor: `<Stack {AgeRecipients(["age1..", "age1.."])}>`.
/// A vault naming its own list does not inherit.
#[derive(Debug, Default, Clone, PartialEq, Eq, Component, Reflect)]
#[reflect(Component, Default)]
pub struct AgeRecipients(pub Vec<AgeRecipient>);

#[cfg(test)]
mod test {
	use crate::prelude::*;

	/// Two recipients with no identity behind them, for the grammar.
	const ALICE: &str =
		"age1ql3z7hjy54pw3hyww5ayyfg7zqgvc7w3j2elw8zmrj2kg5sfn9aqmcac8p";
	const BOB: &str =
		"age1lggyhqrw2nlhcxprm67z43rta597azn8gknawjehu9d9dl0jq3yqqvfafg";

	/// Build `markup` into a world registering the declarations, erroring as
	/// the build does.
	#[cfg(feature = "bsx")]
	fn build(markup: &str) -> Result<World> {
		let mut world = (TemplatePlugin, DocumentPlugin).into_world();
		{
			let registry = world.resource_mut::<AppTypeRegistry>();
			let mut registry = registry.write();
			registry.register::<Vault>();
			registry.register::<AgeRecipients>();
		}
		let nodes = BsxNode::parse_document(markup, &BsxParseConfig::bsx())?;
		world.spawn_template(BsxTemplate::container(
			nodes,
			BsxTemplateRegistry::default(),
		))?;
		world.flush();
		world.xok()
	}

	/// A recipient list authors from markup through the literal parser, on
	/// the tag and on the spread alike, and a typo is an error rather than a
	/// silently empty list.
	#[cfg(feature = "bsx")]
	#[crate::test]
	fn authors_recipients_from_markup() {
		let mut world = build(&format!(
			r#"<Fragment {{AgeRecipients(["{ALICE}"])}}><Vault label="mail" path="mail.toml.age" recipients={{["{ALICE}", "{BOB}"]}}/></Fragment>"#
		))
		.unwrap();
		let vault = world.query::<&Vault>().single(&world).unwrap();
		vault.label.as_str().xpect_eq("mail");
		vault.path.as_str().xpect_eq("mail.toml.age");
		vault
			.recipients
			.iter()
			.map(AgeRecipient::as_str)
			.collect::<Vec<_>>()
			.xpect_eq(vec![ALICE, BOB]);
		world
			.query::<&AgeRecipients>()
			.single(&world)
			.unwrap()
			.0
			.len()
			.xpect_eq(1);
		build(
			r#"<Vault label="x" path="x.toml.age" recipients={["age1typo"]}/>"#,
		)
		.unwrap_err()
		.to_string()
		.xpect_contains("age-keygen");
	}

	#[crate::test]
	fn defaults_to_the_env_vault() {
		let vault = Vault::default();
		vault.label.as_str().xpect_eq(".env");
		vault.path.as_str().xpect_eq(".env.age");
		vault.format().unwrap().xpect_eq(VaultFormat::Env);
		Vault::new("infra/secrets/mail.toml.age")
			.label
			.as_str()
			.xpect_eq("mail.toml.age");
	}
}
