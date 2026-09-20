//! The secrets document: a plaintext index of records plus one age blob per
//! group.

use crate::prelude::*;

/// A secrets document, `secrets.toml` by convention: a plaintext index of
/// records (name, role, note, rotation, modified) under the group each
/// belongs to, and one armored age blob per group, sealed to that group's
/// recipients. Everyone sees which records exist and who may read them;
/// only a group's members read its values; `ls` needs no identity at all.
/// The only cipher is age and the only composition a payload in the
/// document's own format, so the file is a plain toml (or json, or ron)
/// whose sealed values are age files: a blob pasted out of it opens with
/// `age -d` on any laptop.
///
/// The file reads as the index first and the ciphertext last: `[groups.<g>]`
/// holds the recipient list and `[groups.<g>.secrets.<NAME>]` each record's
/// metadata, and one `[sealed]` table at the bottom maps every group name to
/// its blob. A record lives in exactly one group, the map it sits in;
/// `default` is the one a `set` names none for, created on first use with
/// the identity file's own recipients. Humans sit in every group; an agent's
/// recipient sits only in the groups it is granted. A group's list is the
/// source of truth for the next seal: a list edit takes effect on the next
/// [`set`](Self::set) of that group or on [`rekey`](Self::rekey), and until
/// then [`open`](Self::open) reports the group as drifted.
///
/// Inside a blob every record carries its value and a copy of its metadata,
/// and the blob carries the list it was sealed to; the sealed side is the
/// truth and the index its mirror, checked on every open, so a document is
/// edited through [`set`](Self::set) and [`remove`](Self::remove) and never
/// by hand.
///
/// ## Example
///
/// ```
/// # use beet_core::prelude::*;
/// let mut identities = AgeIdentityFile::default();
/// identities.push(AgeIdentity::generate());
/// let mut document = SecretsDocument::default();
/// document
/// 	.set(&identities, "default", "OPENAI_API_KEY", "sk-test", SecretRecord {
/// 		role: Some(SecretRole::EnvVar),
/// 		..default()
/// 	})
/// 	.unwrap();
/// let bytes = document.to_bytes().unwrap();
/// let opened = SecretsDocument::parse(MediaType::Toml, &bytes)
/// 	.unwrap()
/// 	.open(&identities)
/// 	.unwrap();
/// opened.get("OPENAI_API_KEY").unwrap().value.as_str().xpect_eq("sk-test");
/// opened.env_vars().len().xpect_eq(1);
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SecretsDocument {
	/// The format the file and every sealed payload are written in, named
	/// by the file's extension.
	#[serde(skip, default = "SecretsDocument::default_media_type")]
	media_type: MediaType,
	/// Where an export came from, absent on a hand-kept document.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub origin: Option<SecretsOrigin>,
	/// The groups by name: a recipient list and the index of its records.
	#[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
	pub groups: BTreeMap<SmolStr, SecretsGroup>,
	/// The armored age file holding each group's [`SealedGroup`], by group
	/// name; absent until the group's first record lands.
	#[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
	pub sealed: BTreeMap<SmolStr, String>,
}

/// A group: who may read it, and the plaintext index of what it holds.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SecretsGroup {
	/// The recipients the next seal encrypts to.
	pub recipients: Vec<AgeRecipient>,
	/// Every record's plaintext metadata by name, mirrored from the sealed
	/// side on every seal.
	#[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
	pub secrets: BTreeMap<SmolStr, SecretRecord>,
}

impl SecretsGroup {
	/// A group with nothing in it yet.
	pub fn new(recipients: Vec<AgeRecipient>) -> Self {
		Self {
			recipients,
			secrets: default(),
		}
	}

	/// Whether any of `recipients` is listed.
	pub fn lists_any(&self, recipients: &[AgeRecipient]) -> bool {
		recipients
			.iter()
			.any(|recipient| self.recipients.contains(recipient))
	}
}

/// Where an exported document came from: the stack and the provider its
/// records were read out of.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecretsOrigin {
	/// The stack's app name.
	pub app: SmolStr,
	/// The stack's stage.
	pub stage: SmolStr,
	/// The provider's region, where it has one.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub region: Option<SmolStr>,
	/// The secret store provider the records were read from.
	pub provider: SmolStr,
	/// When the export was taken.
	#[serde(with = "super::secret_record::iso8601")]
	pub exported: Timestamp,
}

/// What [`SecretsDocument::rekey`] did: the groups re-sealed and the ones
/// this identity could not open.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct RekeyReport {
	/// Re-sealed to their current list.
	pub rekeyed: Vec<SmolStr>,
	/// Not opened by this identity, so left as they were.
	pub locked: Vec<SmolStr>,
}

impl Default for SecretsDocument {
	fn default() -> Self { Self::new(Self::default_media_type()) }
}

impl SecretsDocument {
	/// The file a document lives in by convention, beside an entry.
	pub const DEFAULT_PATH: &'static str = "secrets.toml";
	/// The conventional file's stem, so the test runner finds
	/// `secrets.<format>` from any crate's directory.
	pub const FILE_STEM: &'static str = "secrets";
	/// The group a record lands in when none is named.
	pub const DEFAULT_GROUP: &'static str = "default";

	/// An empty document in `media_type`.
	pub fn new(media_type: MediaType) -> Self {
		Self {
			media_type,
			origin: None,
			groups: default(),
			sealed: default(),
		}
	}

	/// The format the conventional path names.
	pub fn default_media_type() -> MediaType { MediaType::Toml }

	/// The format the file and its sealed payloads are written in.
	pub fn media_type(&self) -> MediaType { self.media_type.clone() }

	/// The format `path` names by its extension: `secrets.toml` is toml,
	/// `x.json` json, `x.ron` ron. Errors on any other extension.
	pub fn media_type_of(path: &str) -> Result<MediaType> {
		match SmolPath::new(path).media_type() {
			Some(
				media_type @ (MediaType::Toml
				| MediaType::Json
				| MediaType::Ron),
			) => media_type.xok(),
			_ => bevybail!(
				"`{path}` is not a secrets document: its extension names the \
				format, `toml`, `json` or `ron`, ie `{}`",
				Self::DEFAULT_PATH
			),
		}
	}

	/// Parse a document in `media_type` and validate it: every group lists
	/// a recipient, every sealed blob has its group, every record name is
	/// well formed and in one group.
	pub fn parse(media_type: MediaType, bytes: &[u8]) -> Result<Self> {
		let mut document: Self = media_type.deserialize(bytes)?;
		document.media_type = media_type;
		document.validate()?;
		document.xok()
	}

	/// The document serialized in its media type, the index plaintext and
	/// every blob armored.
	pub fn to_bytes(&self) -> Result<Vec<u8>> {
		self.media_type.serialize(self)
	}

	/// The structural rules a parsed document must satisfy, see
	/// [`parse`](Self::parse).
	pub fn validate(&self) -> Result<()> {
		let mut seen = HashMap::<&SmolStr, &SmolStr>::default();
		for (group_name, group) in &self.groups {
			if group_name.is_empty() {
				bevybail!("a group has no name");
			}
			if group.recipients.is_empty() {
				bevybail!(
					"group `{group_name}` lists no recipients: every group names \
					who may read it"
				);
			}
			for name in group.secrets.keys() {
				Self::validate_name(name)?;
				if let Some(other) = seen.insert(name, group_name) {
					bevybail!(
						"record `{name}` is in both group `{other}` and group \
						`{group_name}`: a name lives in one group"
					);
				}
			}
		}
		for group_name in self.sealed.keys() {
			if !self.groups.contains_key(group_name) {
				bevybail!(
					"a blob is sealed for group `{group_name}`, which the \
					document does not declare"
				);
			}
		}
		Ok(())
	}

	/// The names of every group, in order.
	pub fn group_names(&self) -> impl Iterator<Item = &SmolStr> {
		self.groups.keys()
	}

	/// Every record's group, name and index entry, grouped then named in
	/// order.
	pub fn records(
		&self,
	) -> impl Iterator<Item = (&SmolStr, &SmolStr, &SecretRecord)> {
		self.groups.iter().flat_map(|(group, records)| {
			records
				.secrets
				.iter()
				.map(move |(name, record)| (group, name, record))
		})
	}

	/// The group holding `name` and its index entry, `None` for no such
	/// record.
	pub fn record(&self, name: &str) -> Option<(&SmolStr, &SecretRecord)> {
		self.groups.iter().find_map(|(group, records)| {
			records.secrets.get(name).map(|record| (group, record))
		})
	}

	/// The group holding `name`.
	pub fn group_of(&self, name: &str) -> Option<&SmolStr> {
		self.record(name).map(|(group, _)| group)
	}

	/// Whether any group holds `name`.
	pub fn contains(&self, name: &str) -> bool { self.record(name).is_some() }

	/// How many records the index holds across every group.
	pub fn record_count(&self) -> usize {
		self.groups.values().map(|group| group.secrets.len()).sum()
	}

	/// Whether `group` has a blob sealed yet.
	pub fn is_sealed(&self, group: &str) -> bool {
		self.sealed.contains_key(group)
	}

	/// Whether any identity in `identities` is listed in `group`.
	pub fn is_member(&self, group: &str, identities: &AgeIdentityFile) -> bool {
		self.groups
			.get(group)
			.is_some_and(|group| group.lists_any(&identities.recipients()))
	}

	/// Open every group `identities` can, verifying each against the index,
	/// into one map. A group that does not open is named by why: locked
	/// (none of these recipients is listed), or pending (listed, but sealed
	/// before it was added, so a member must `rekey`). Opening never
	/// consults the list to decide: age is tried on every blob, and the
	/// list only names why a blob stayed shut. Errors when an opened group's
	/// records differ from the index (decision: the sealed side is the
	/// truth), naming every difference.
	pub fn open(&self, identities: &AgeIdentityFile) -> Result<OpenSecrets> {
		let own = identities.recipients();
		let mut open = OpenSecrets::default();
		for (name, group) in &self.groups {
			if !self.is_sealed(name) {
				// nothing sealed yet: a member may write it, a stranger not
				match group.lists_any(&own) {
					true => open.opened.push(name.clone()),
					false => open.locked.push(name.clone()),
				}
				continue;
			}
			let Some(sealed) = self.try_open_group(name, identities)? else {
				match group.lists_any(&own) {
					true => open.pending.push(name.clone()),
					false => open.locked.push(name.clone()),
				};
				continue;
			};
			if !same_recipients(&sealed.recipients, &group.recipients) {
				open.drifted.push(name.clone());
			}
			for (record_name, sealed_record) in sealed.secrets {
				open.secrets.insert(record_name.clone(), Secret {
					name: record_name,
					group: name.clone(),
					value: sealed_record.value,
					record: sealed_record.record,
				});
			}
			open.opened.push(name.clone());
		}
		open.xok()
	}

	/// Write one record into `group`: the group is opened (created with the
	/// identity file's own recipients when new), the record merged in with
	/// `modified` now unless given, the blob re-sealed to the group's
	/// current list and the index updated. A record already in another
	/// group moves, removed from the old blob and added to the new. Errors
	/// naming the group when `identities` cannot open it.
	pub fn set(
		&mut self,
		identities: &AgeIdentityFile,
		group: &str,
		name: &str,
		value: &str,
		record: SecretRecord,
	) -> Result<()> {
		Self::validate_name(name)?;
		if group.is_empty() {
			bevybail!("a group has no name");
		}
		if !self.groups.contains_key(group) {
			let recipients = identities.recipients();
			if recipients.is_empty() {
				bevybail!(
					"cannot create group `{group}`: the identity file holds \
					no identity to list as its recipient"
				);
			}
			info!(
				"creating group `{group}` with this identity file's {} \
				recipient(s); edit its list in the document to add readers",
				recipients.len()
			);
			self.groups
				.insert(SmolStr::new(group), SecretsGroup::new(recipients));
		}
		// a record moving between groups leaves its old blob first
		if let Some(previous) = self
			.group_of(name)
			.cloned()
			.filter(|previous| previous != group)
		{
			let mut sealed = self.open_group(&previous, identities)?;
			sealed.secrets.remove(name);
			self.seal_group(&previous, sealed)?;
		}
		let mut sealed = self.open_group(group, identities)?;
		let record = SecretRecord {
			modified: record.modified.or_else(|| Timestamp::try_now().ok()),
			..record
		};
		sealed.secrets.insert(SmolStr::new(name), SealedRecord {
			value: SmolStr::new(value),
			record,
		});
		self.seal_group(group, sealed)
	}

	/// Remove one record, re-sealing its group, answering the group it was
	/// in and its index entry. Errors when the record does not exist or
	/// `identities` cannot open its group.
	pub fn remove(
		&mut self,
		identities: &AgeIdentityFile,
		name: &str,
	) -> Result<(SmolStr, SecretRecord)> {
		let group = self
			.group_of(name)
			.cloned()
			.ok_or_else(|| bevyhow!("no record `{name}` in the document"))?;
		let mut sealed = self.open_group(&group, identities)?;
		let removed = sealed
			.secrets
			.remove(name)
			.map(|sealed| sealed.record)
			.ok_or_else(|| {
			bevyhow!("record `{name}` is in the index but not sealed")
		})?;
		self.seal_group(&group, sealed)?;
		(group, removed).xok()
	}

	/// Replace `group` wholesale: its list becomes `recipients` and its blob
	/// holds exactly `records`, sealed once. A fresh seal opens nothing, so
	/// no identity is needed: an export or a restore writes a document it
	/// may not itself be able to read. A record of the same name in another
	/// group is an error, since a name lives in one group.
	pub fn seal_records(
		&mut self,
		group: &str,
		recipients: Vec<AgeRecipient>,
		records: impl IntoIterator<Item = (SmolStr, SmolStr, SecretRecord)>,
	) -> Result<()> {
		if group.is_empty() {
			bevybail!("a group has no name");
		}
		if recipients.is_empty() {
			bevybail!(
				"group `{group}` lists no recipients: every group names who \
				may read it"
			);
		}
		let mut payload = SealedGroup::default();
		for (name, value, record) in records {
			Self::validate_name(&name)?;
			if let Some(existing) =
				self.group_of(&name).filter(|existing| *existing != group)
			{
				bevybail!(
					"record `{name}` is already in group `{existing}`: a name \
					lives in one group"
				);
			}
			payload.secrets.insert(name, SealedRecord { value, record });
		}
		self.groups
			.insert(SmolStr::new(group), SecretsGroup::new(recipients));
		self.seal_group(group, payload)
	}

	/// Re-seal every group `identities` can open to its current recipient
	/// list, naming the ones it cannot. A group with nothing sealed needs
	/// no rekey and is neither.
	pub fn rekey(
		&mut self,
		identities: &AgeIdentityFile,
	) -> Result<RekeyReport> {
		let mut report = RekeyReport::default();
		for name in self.sealed.keys().cloned().collect::<Vec<_>>() {
			match self.try_open_group(&name, identities)? {
				Some(sealed) => {
					self.seal_group(&name, sealed)?;
					report.rekeyed.push(name);
				}
				None => report.locked.push(name),
			}
		}
		report.xok()
	}

	/// [`try_open_group`](Self::try_open_group), erroring by name when
	/// `identities` cannot open the group.
	fn open_group(
		&self,
		group: &str,
		identities: &AgeIdentityFile,
	) -> Result<SealedGroup> {
		self.try_open_group(group, identities)?.ok_or_else(|| {
			bevyhow!(
				"this identity cannot open group `{group}`: it was sealed to \
				other recipients; a member adds a recipient to the list and \
				runs `secrets/rekey`"
			)
		})
	}

	/// Decrypt and verify one group's blob: `None` when `identities` cannot
	/// open it, an empty payload when nothing is sealed yet, an error when
	/// the blob disagrees with the index.
	fn try_open_group(
		&self,
		group: &str,
		identities: &AgeIdentityFile,
	) -> Result<Option<SealedGroup>> {
		let Some(sealed) = self.sealed.get(group) else {
			return Some(SealedGroup::default()).xok();
		};
		if identities.is_empty() {
			return None.xok();
		}
		match identities.decrypt(sealed.as_bytes()) {
			Ok(payload) => self.verify_group(group, &payload).map(Some),
			Err(_) => None.xok(),
		}
	}

	/// Parse a decrypted payload and check it against the group's index:
	/// every sealed record has its index entry with the same metadata, and
	/// every index entry is sealed. Errors naming every difference.
	fn verify_group(&self, group: &str, payload: &[u8]) -> Result<SealedGroup> {
		let sealed: SealedGroup = self.media_type.deserialize(payload)?;
		let index = self
			.groups
			.get(group)
			.map(|group| &group.secrets)
			.ok_or_else(|| bevyhow!("no group `{group}`"))?;
		let mut problems = Vec::new();
		for (name, sealed_record) in &sealed.secrets {
			match index.get(name) {
				None => problems.push(format!(
					"`{name}` is sealed in `{group}` but missing from the index"
				)),
				Some(entry) if *entry != sealed_record.record => {
					problems.push(format!(
						"`{name}`: the index entry differs from its sealed copy"
					))
				}
				Some(_) => {}
			}
		}
		for name in index
			.keys()
			.filter(|name| !sealed.secrets.contains_key(*name))
		{
			problems.push(format!(
				"`{name}` is in the index under `{group}` but not sealed there"
			));
		}
		if !problems.is_empty() {
			bevybail!(
				"group `{group}` does not match the index (a document is \
				edited through `secrets/set`, never by hand): {}",
				problems.join("; ")
			);
		}
		sealed.xok()
	}

	/// Seal `payload` to `group`'s current list and mirror its records into
	/// the group's index.
	fn seal_group(
		&mut self,
		group: &str,
		mut payload: SealedGroup,
	) -> Result<()> {
		let entry = self
			.groups
			.get_mut(group)
			.ok_or_else(|| bevyhow!("no group `{group}`"))?;
		payload.recipients = entry.recipients.clone();
		let ciphertext = AgeRecipient::encrypt(
			&payload.recipients,
			&self.media_type.serialize(&payload)?,
		)?;
		entry.secrets = payload
			.secrets
			.into_iter()
			.map(|(name, sealed_record)| (name, sealed_record.record))
			.collect();
		self.sealed.insert(SmolStr::new(group), ciphertext);
		Ok(())
	}

	/// A record name is non-empty and carries no whitespace or `=`, so it
	/// is an env var name or a secret label as is.
	fn validate_name(name: &str) -> Result<()> {
		match name.is_empty()
			|| name.contains(char::is_whitespace)
			|| name.contains('=')
		{
			true => bevybail!(
				"`{name}` is not a record name: one is non-empty with no \
				whitespace or `=`, ie `OPENAI_API_KEY` or `dkim-example-com`"
			),
			false => Ok(()),
		}
	}
}

/// Whether two recipient lists name the same set.
fn same_recipients(left: &[AgeRecipient], right: &[AgeRecipient]) -> bool {
	left.iter().collect::<HashSet<_>>() == right.iter().collect::<HashSet<_>>()
}

#[cfg(test)]
mod test {
	use super::*;

	/// Group names as a document lists them.
	fn names(items: &[&str]) -> Vec<SmolStr> {
		items.iter().map(|item| SmolStr::new(item)).collect()
	}

	/// An identity and the file holding it alone.
	pub(super) fn human() -> (AgeIdentity, AgeIdentityFile) {
		let identity = AgeIdentity::generate();
		let mut file = AgeIdentityFile::default();
		file.push(identity.clone());
		(identity, file)
	}

	/// A record with a fixed `modified`, so a document serializes the same
	/// way twice.
	pub(super) fn record(role: Option<SecretRole>, note: &str) -> SecretRecord {
		SecretRecord {
			role,
			note: Some(note.into()),
			modified: Some(Timestamp::parse_date("2026-09-18").unwrap()),
			..default()
		}
	}

	/// A document with `default` (pete) and `agents` (pete and the agent)
	/// holding three records, and the two identity files.
	pub(super) fn two_groups()
	-> (SecretsDocument, AgeIdentityFile, AgeIdentityFile) {
		let (pete, pete_file) = human();
		let (agent, agent_file) = human();
		let mut document = SecretsDocument::default();
		document.groups.insert(
			"agents".into(),
			SecretsGroup::new(vec![pete.to_recipient(), agent.to_recipient()]),
		);
		document
			.set(
				&pete_file,
				"default",
				"OPENAI_API_KEY",
				"sk-test",
				record(Some(SecretRole::EnvVar), "billing account"),
			)
			.unwrap();
		document
			.set(
				&pete_file,
				"agents",
				"CF_API_TOKEN",
				"cf-test",
				record(Some(SecretRole::EnvVar), "dns and workers"),
			)
			.unwrap();
		document
			.set(
				&pete_file,
				"default",
				"dkim-example-com",
				"-----BEGIN PRIVATE KEY-----",
				record(None, "the signing key"),
			)
			.unwrap();
		(document, pete_file, agent_file)
	}

	/// The document's text with the per-run recipients and blobs cut, so it
	/// snapshots.
	pub(super) fn redacted(document: &SecretsDocument) -> String {
		let text = String::from_utf8(document.to_bytes().unwrap()).unwrap();
		let mut in_blob = false;
		text.lines()
			.filter(|line| {
				let armor = line.starts_with("-----");
				in_blob = (in_blob || armor) && !(in_blob && armor);
				!in_blob
					&& !armor && !line.starts_with("recipients")
					&& !line.trim_start().starts_with("\"age1")
					&& line.trim() != "]"
			})
			.collect::<Vec<_>>()
			.join("\n")
	}

	/// The index reads first and the blobs last, and the whole document
	/// round-trips.
	#[crate::test]
	fn roundtrips_and_snapshots() {
		let (document, pete, _) = two_groups();
		let bytes = document.to_bytes().unwrap();
		let text = String::from_utf8(bytes.clone()).unwrap();
		redacted(&document).xpect_snapshot();
		text.as_str()
			.xpect_contains("[groups.default.secrets.OPENAI_API_KEY]")
			.xpect_contains("[groups.agents.secrets.CF_API_TOKEN]")
			.xpect_contains(
				"[sealed]\nagents = \"\"\"\n-----BEGIN AGE ENCRYPTED FILE-----",
			)
			.xpect_contains("modified = \"2026-09-18T00:00:00.000Z\"")
			.xnot()
			.xpect_contains("sk-test");
		// the ciphertext sits below the whole index
		let index_end = text.find("[sealed]").unwrap();
		text[index_end..].xnot().xpect_contains("[groups.");
		let parsed = SecretsDocument::parse(MediaType::Toml, &bytes).unwrap();
		parsed.xpect_eq(document.clone());
		document.record_count().xpect_eq(3);
		document
			.group_of("CF_API_TOKEN")
			.unwrap()
			.as_str()
			.xpect_eq("agents");
		document.contains("nope").xpect_false();
		let opened = parsed.open(&pete).unwrap();
		opened.secrets.len().xpect_eq(3);
		opened.opened.xpect_eq(names(&["agents", "default"]));
		let key = opened.get("OPENAI_API_KEY").unwrap();
		key.value.as_str().xpect_eq("sk-test");
		key.record.role.xpect_eq(Some(SecretRole::EnvVar));
		key.record
			.note
			.clone()
			.unwrap()
			.as_str()
			.xpect_eq("billing account");
		key.group.as_str().xpect_eq("default");
		opened
			.get("CF_API_TOKEN")
			.unwrap()
			.group
			.as_str()
			.xpect_eq("agents");
	}

	#[crate::test]
	fn json_documents_seal_json() {
		let (_, pete) = human();
		let mut document = SecretsDocument::new(MediaType::Json);
		document.set(&pete, "default", "A", "1", default()).unwrap();
		let bytes = document.to_bytes().unwrap();
		bytes.starts_with(b"{").xpect_true();
		let payload =
			pete.decrypt(document.sealed["default"].as_bytes()).unwrap();
		payload.starts_with(b"{").xpect_true();
		SecretsDocument::parse(MediaType::Json, &bytes)
			.unwrap()
			.open(&pete)
			.unwrap()
			.get("A")
			.unwrap()
			.value
			.as_str()
			.xpect_eq("1");
	}

	#[crate::test]
	fn media_type_of_path() {
		SecretsDocument::media_type_of("secrets.toml")
			.unwrap()
			.xpect_eq(MediaType::Toml);
		SecretsDocument::media_type_of("infra/x.json")
			.unwrap()
			.xpect_eq(MediaType::Json);
		SecretsDocument::media_type_of("cert.pem.age")
			.unwrap_err()
			.to_string()
			.xpect_contains("format");
	}

	/// The agent opens `agents` alone, and its env vars are that group's.
	#[crate::test]
	fn a_member_of_one_group_reads_that_group() {
		let (document, _, agent) = two_groups();
		let opened = document.open(&agent).unwrap();
		opened.opened.xpect_eq(names(&["agents"]));
		opened.locked.xpect_eq(names(&["default"]));
		opened.pending.xpect_eq(names(&[]));
		opened
			.env_vars()
			.xpect_eq(vec![("CF_API_TOKEN".into(), "cf-test".into())]);
		opened.get("OPENAI_API_KEY").xpect_none();
	}

	#[crate::test]
	fn set_by_a_non_member_is_refused() {
		let (mut document, _, agent) = two_groups();
		document
			.set(&agent, "default", "OPENAI_API_KEY", "sk-new", default())
			.unwrap_err()
			.to_string()
			.xpect_contains("cannot open group `default`");
		// the group it is in is fine
		document
			.set(&agent, "agents", "CF_API_TOKEN", "cf-new", default())
			.unwrap();
	}

	#[crate::test]
	fn moves_a_record_between_groups() {
		let (mut document, pete, agent) = two_groups();
		document
			.set(
				&pete,
				"agents",
				"OPENAI_API_KEY",
				"sk-test",
				record(Some(SecretRole::EnvVar), "moved"),
			)
			.unwrap();
		document
			.group_of("OPENAI_API_KEY")
			.unwrap()
			.as_str()
			.xpect_eq("agents");
		document.groups["default"]
			.secrets
			.contains_key("OPENAI_API_KEY")
			.xpect_false();
		let opened = document.open(&agent).unwrap();
		opened
			.get("OPENAI_API_KEY")
			.unwrap()
			.group
			.as_str()
			.xpect_eq("agents");
		// and it left `default`, which pete still verifies clean
		document.open(&pete).unwrap().secrets.len().xpect_eq(3);
	}

	#[crate::test]
	fn remove_prunes_and_reseals() {
		let (mut document, pete, _) = two_groups();
		let (group, removed) =
			document.remove(&pete, "OPENAI_API_KEY").unwrap();
		group.as_str().xpect_eq("default");
		removed.note.unwrap().as_str().xpect_eq("billing account");
		document.contains("OPENAI_API_KEY").xpect_false();
		document.open(&pete).unwrap().secrets.len().xpect_eq(2);
		document
			.remove(&pete, "OPENAI_API_KEY")
			.unwrap_err()
			.to_string()
			.xpect_contains("no record");
	}

	/// Each way of editing the index by hand is refused by name.
	#[crate::test]
	fn index_tampering_is_refused() {
		let (document, pete, _) = two_groups();
		let index = |document: &mut SecretsDocument, group: &str| {
			document.groups.get_mut(group).unwrap().secrets.clone()
		};
		// a changed role
		let mut tampered = document.clone();
		tampered
			.groups
			.get_mut("default")
			.unwrap()
			.secrets
			.get_mut("dkim-example-com")
			.unwrap()
			.role = Some(SecretRole::EnvVar);
		tampered
			.open(&pete)
			.unwrap_err()
			.to_string()
			.xpect_contains("`dkim-example-com`: the index entry differs");
		// a renamed record
		let mut tampered = document.clone();
		let mut secrets = index(&mut tampered, "default");
		let record = secrets.remove("OPENAI_API_KEY").unwrap();
		secrets.insert("OPENAI_KEY".into(), record);
		tampered.groups.get_mut("default").unwrap().secrets = secrets;
		tampered
			.open(&pete)
			.unwrap_err()
			.to_string()
			.xpect_contains(
				"`OPENAI_API_KEY` is sealed in `default` but missing",
			)
			.xpect_contains(
				"`OPENAI_KEY` is in the index under `default` but not sealed",
			);
		// a deleted record
		let mut tampered = document.clone();
		tampered
			.groups
			.get_mut("agents")
			.unwrap()
			.secrets
			.remove("CF_API_TOKEN");
		tampered
			.open(&pete)
			.unwrap_err()
			.to_string()
			.xpect_contains("`CF_API_TOKEN` is sealed in `agents` but missing");
		// a record moved to another group's map by hand
		let mut tampered = document.clone();
		let record = tampered
			.groups
			.get_mut("agents")
			.unwrap()
			.secrets
			.remove("CF_API_TOKEN")
			.unwrap();
		tampered
			.groups
			.get_mut("default")
			.unwrap()
			.secrets
			.insert("CF_API_TOKEN".into(), record);
		// the first group opened names it (the index maps are read in order)
		tampered
			.open(&pete)
			.unwrap_err()
			.to_string()
			.xpect_contains("`CF_API_TOKEN` is sealed in `agents` but missing");
	}

	/// A list edited by hand is reported as drift, and `rekey` clears it
	/// while letting the new recipient in.
	#[crate::test]
	fn rekey_after_a_list_edit() {
		let (mut document, pete, _) = two_groups();
		let (alice, alice_file) = human();
		document
			.groups
			.get_mut("default")
			.unwrap()
			.recipients
			.push(alice.to_recipient());
		document
			.open(&pete)
			.unwrap()
			.drifted
			.xpect_eq(names(&["default"]));
		// listed, but sealed before she was added
		let opened = document.open(&alice_file).unwrap();
		opened.pending.xpect_eq(names(&["default"]));
		opened.locked.xpect_eq(names(&["agents"]));
		let report = document.rekey(&pete).unwrap();
		report.rekeyed.xpect_eq(names(&["agents", "default"]));
		report.locked.xpect_eq(names(&[]));
		document.open(&pete).unwrap().drifted.xpect_eq(names(&[]));
		document
			.open(&alice_file)
			.unwrap()
			.get("OPENAI_API_KEY")
			.unwrap()
			.value
			.as_str()
			.xpect_eq("sk-test");
		// the agent can rekey only its own group
		let (mut document, _, agent_file) = two_groups();
		let report = document.rekey(&agent_file).unwrap();
		report.rekeyed.xpect_eq(names(&["agents"]));
		report.locked.xpect_eq(names(&["default"]));
	}

	/// The escape hatch: a blob pasted out of the document is a plain age
	/// file whose payload is toml.
	#[crate::test]
	fn a_blob_opens_to_toml() {
		let (document, pete, _) = two_groups();
		let blob = document.sealed["default"].clone();
		blob.as_str()
			.xpect_starts_with("-----BEGIN AGE ENCRYPTED FILE-----");
		let payload = pete
			.identities()
			.next()
			.unwrap()
			.decrypt(blob.as_bytes())
			.unwrap();
		let text = String::from_utf8(payload).unwrap();
		text.as_str()
			.xpect_contains("[secrets.OPENAI_API_KEY]")
			.xpect_contains("value = \"sk-test\"")
			.xpect_contains("role = \"env_var\"");
		toml::from_str::<toml::Table>(&text).unwrap();
	}

	#[crate::test]
	fn parse_validates() {
		let (document, ..) = two_groups();
		// a name in two groups
		let mut bad = document.clone();
		let record = bad.groups["default"].secrets["OPENAI_API_KEY"].clone();
		bad.groups
			.get_mut("agents")
			.unwrap()
			.secrets
			.insert("OPENAI_API_KEY".into(), record);
		SecretsDocument::parse(MediaType::Toml, &bad.to_bytes().unwrap())
			.unwrap_err()
			.to_string()
			.xpect_contains("in both group `agents` and group `default`");
		// a blob for no group
		let mut bad = document.clone();
		let blob = bad.sealed["default"].clone();
		bad.sealed.insert("nope".into(), blob);
		SecretsDocument::parse(MediaType::Toml, &bad.to_bytes().unwrap())
			.unwrap_err()
			.to_string()
			.xpect_contains("sealed for group `nope`");
		// a stray table (a hand edit, an older shape) is refused by name
		SecretsDocument::parse(
			MediaType::Toml,
			b"[secrets.A]\nrole = \"env_var\"\n",
		)
		.unwrap_err()
		.to_string()
		.xpect_contains("unknown field `secrets`");
		let mut bad = document.clone();
		bad.groups.get_mut("agents").unwrap().recipients.clear();
		SecretsDocument::parse(MediaType::Toml, &bad.to_bytes().unwrap())
			.unwrap_err()
			.to_string()
			.xpect_contains("lists no recipients");
		let (_, pete) = human();
		SecretsDocument::default()
			.set(&pete, "default", "BAD NAME", "x", default())
			.unwrap_err()
			.to_string()
			.xpect_contains("not a record name");
		// a fresh document with no identity to seed `default`
		SecretsDocument::default()
			.set(&AgeIdentityFile::default(), "default", "A", "x", default())
			.unwrap_err()
			.to_string()
			.xpect_contains("no identity");
	}

	/// A group sealed in one go needs no identity, reads back for a listed
	/// recipient with every record and its metadata, and replaces what the
	/// group held before.
	#[crate::test]
	fn seals_records_in_one_go() {
		let (mut document, pete, agent) = two_groups();
		let (alice, alice_file) = human();
		document
			.seal_records("default", vec![alice.to_recipient()], [
				(
					"dkim-example-com".into(),
					"-----BEGIN PRIVATE KEY-----".into(),
					record(None, "the signing key"),
				),
				("mail-tlsa".into(), "abc".into(), record(None, "the pin")),
			])
			.unwrap();
		// the old `default` record is gone, `agents` untouched
		document.contains("OPENAI_API_KEY").xpect_false();
		document.record_count().xpect_eq(3);
		let opened = document.open(&alice_file).unwrap();
		opened.opened.xpect_eq(names(&["default"]));
		opened
			.get("mail-tlsa")
			.unwrap()
			.value
			.as_str()
			.xpect_eq("abc");
		opened
			.get("dkim-example-com")
			.unwrap()
			.record
			.note
			.clone()
			.unwrap()
			.as_str()
			.xpect_eq("the signing key");
		// pete is no longer listed in `default`
		document
			.open(&pete)
			.unwrap()
			.locked
			.xpect_eq(names(&["default"]));
		document
			.open(&agent)
			.unwrap()
			.get("CF_API_TOKEN")
			.xpect_some();
		// a name already in another group is refused
		document
			.seal_records("other", vec![alice.to_recipient()], [(
				"CF_API_TOKEN".into(),
				"x".into(),
				default(),
			)])
			.unwrap_err()
			.to_string()
			.xpect_contains("already in group `agents`");
		document
			.seal_records("other", vec![], [])
			.unwrap_err()
			.to_string()
			.xpect_contains("no recipients");
	}

	/// With no identity at all every sealed group is locked and the index
	/// still reads.
	#[crate::test]
	fn opens_nothing_without_an_identity() {
		let (document, ..) = two_groups();
		let opened = document.open(&AgeIdentityFile::default()).unwrap();
		opened.secrets.is_empty().xpect_true();
		opened.locked.xpect_eq(names(&["agents", "default"]));
		document.record_count().xpect_eq(3);
	}
}
