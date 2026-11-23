//! The deploy half of a stack's ssh certificate authority: minting it before
//! the apply that boots a box, and signing with it every time a deploy reaches
//! one. The render half, and why there is one authority per stack, is
//! [`SshAuthority`]'s module.
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use ssh_key::Algorithm;
use ssh_key::Certificate;
use ssh_key::HashAlg;
use ssh_key::LineEnding;
use ssh_key::PrivateKey;
use ssh_key::PublicKey;
use ssh_key::certificate::Builder;
use ssh_key::certificate::CertType;

/// `<EnsureSshAuthority/>` — mint the stack's ssh certificate authority, and
/// every box's host key and certificate, before the apply that boots a box
/// trusting them.
///
/// Create-IF-MISSING, like [`EnsureSecret`]: a box trusts the authority it
/// booted with, so a re-minted authority locks every running box of the
/// stack out until it is replaced. The public halves are converged beside
/// their keys every deploy, since they are derived, and a host certificate is
/// re-signed only when it no longer holds: another key, another authority or
/// another name.
///
/// Declared ahead of the `<TofuApply/>` in a stack whose boxes trust it (a
/// [`StalwartBlock`], a [`LightsailBlock`]), for the same reason
/// `<EnsureDkimKey/>` is: the box's `user_data` reads the user authority's
/// public half as a content variable, which refuses to render until it exists.
#[action]
#[derive(Debug, Component, Reflect)]
#[reflect(Component, Default)]
pub async fn EnsureSshAuthority(
	cx: ActionContext<Request>,
) -> Result<Outcome<Request, Response>> {
	let (store, hosts) = cx
		.caller
		.with_state::<SshHostQuery, _>(|entity, query| query.resolve(entity))
		.await??;
	let authority = SshAuthority::ensure(&store).await?;
	for host in &hosts {
		authority.ensure_host(&store, host).await?;
	}
	Pass(cx.input).xok()
}

impl SshAuthority {
	/// How long a deploy's login certificate is good for. Only the login
	/// checks it, so a session outlives it; a connection re-signs before
	/// each login once its certificate is half spent
	/// ([`SshConnection`]).
	pub const LOGIN_VALIDITY: Duration = Duration::from_secs(10 * 60);

	/// How far back a certificate's validity starts, so a box whose clock
	/// runs a little behind this machine's still admits it.
	const CLOCK_SKEW: Duration = Duration::from_secs(60);

	/// What a login certificate permits beyond the shell: a pty for an
	/// operator's session and port forwarding for the mail provision's
	/// tunnel. No agent or X11 forwarding.
	const LOGIN_EXTENSIONS: [&'static str; 2] =
		["permit-port-forwarding", "permit-pty"];

	/// Mint whatever of the stack's authority is missing and converge both
	/// public halves, answering the two private keys.
	pub async fn ensure(store: &SecretStore) -> Result<SshAuthorityKeys> {
		let user = Self::ensure_ca(
			store,
			&Self::user_ca(),
			&Self::user_ca_public(),
			"user",
		)
		.await?;
		let host = Self::ensure_ca(
			store,
			&Self::host_ca(),
			&Self::host_ca_public(),
			"host",
		)
		.await?;
		SshAuthorityKeys { user, host }.xok()
	}

	/// What a connection needs, read rather than minted: an authority the
	/// stack has not minted is an error naming the step that mints it.
	pub async fn access(store: &SecretStore) -> Result<SshAccess> {
		let hint = || {
			"the deploy's <EnsureSshAuthority/> mints it, before the apply \
			 that boots a box trusting it"
				.to_string()
		};
		let user_ca = store
			.require(&Self::user_ca(), hint)
			.await?
			.xmap(|key| PrivateKey::from_openssh(key.as_bytes()))?;
		let host_ca = store
			.require(&Self::host_ca_public(), hint)
			.await?
			.xmap(|key| PublicKey::from_openssh(&key))?;
		SshAccess { user_ca, host_ca }.xok()
	}

	/// One authority's key, created if missing, with its public half
	/// converged beside it.
	async fn ensure_ca(
		store: &SecretStore,
		secret: &SecretRef,
		public: &SecretRef,
		role: &str,
	) -> Result<PrivateKey> {
		let (encoded, minted) = store
			.ensure(
				secret,
				Some(&format!(
					"the stack's ssh {role} certificate authority (ed25519)"
				)),
				Self::authority_roll(),
				async || {
					Self::generate_key().and_then(|key| Self::encode(&key))
				},
			)
			.await?;
		if minted {
			info!("minted the ssh {role} authority {}", store.address(secret));
		}
		let key = PrivateKey::from_openssh(encoded.as_bytes())?;
		let derived = key.public_key().to_openssh()?;
		if store
			.converge(
				public,
				&derived,
				Some(&format!(
					"the public half of the stack's ssh {role} certificate \
					 authority"
				)),
				Some(SecretRoll::manual(
					"derived from the authority's key: roll that",
				)),
			)
			.await?
		{
			info!("published the ssh {role} authority's public half");
		}
		key.xok()
	}

	/// How an authority rolls: by hand, since every box of the stack trusts
	/// the one it booted with.
	fn authority_roll() -> SecretRoll {
		SecretRoll::manual(
			"delete the key and its public half, deploy, then replace every \
			 box of the stack: each trusts the authority it booted with",
		)
	}

	/// A fresh ed25519 key.
	pub(crate) fn generate_key() -> Result<PrivateKey> {
		PrivateKey::random(&mut SecureRandom::new()?, Algorithm::Ed25519)?.xok()
	}

	/// A private key as the store holds it, OpenSSH encoded.
	pub(crate) fn encode(key: &PrivateKey) -> Result<String> {
		key.to_openssh(LineEnding::LF)?.to_string().xok()
	}

	/// A certificate for `public` from `ca`, of `cert_type`, valid for
	/// `principals` from a minute ago until `valid_before`.
	fn sign(
		ca: &PrivateKey,
		public: &PublicKey,
		cert_type: CertType,
		key_id: &str,
		principals: &[&str],
		valid_before: u64,
		extensions: &[&str],
	) -> Result<Certificate> {
		let now = time_ext::now();
		let valid_after = now.saturating_sub(Self::CLOCK_SKEW).as_secs();
		let mut builder = Builder::new_with_random_nonce(
			&mut SecureRandom::new()?,
			public.key_data().clone(),
			valid_after,
			valid_before,
		)?;
		builder
			.serial(now.as_millis() as u64)?
			.cert_type(cert_type)?
			.key_id(key_id)?;
		for principal in principals {
			builder.valid_principal(*principal)?;
		}
		for extension in extensions {
			builder.extension(*extension, "")?;
		}
		builder.sign(ca)?.xok()
	}
}

/// The stack's two authorities, private halves in hand: what
/// [`EnsureSshAuthority`] signs a box's host certificate with.
pub struct SshAuthorityKeys {
	user: PrivateKey,
	host: PrivateKey,
}

impl std::fmt::Debug for SshAuthorityKeys {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("SshAuthorityKeys")
			.field("user", &self.user.fingerprint(HashAlg::Sha256))
			.field("host", &self.host.fingerprint(HashAlg::Sha256))
			.finish()
	}
}

impl SshAuthorityKeys {
	/// The box's host key, created if missing, and its certificate, signed
	/// again only when the stored one no longer holds for this key, this
	/// authority and this name.
	///
	/// The certificate never expires: it rides the store, and the box reads
	/// it once at first boot, so an expiry would be an outage on a date
	/// nobody chose. Rolling the host authority is what retires it.
	pub async fn ensure_host(
		&self,
		store: &SecretStore,
		host: &SshHost,
	) -> Result {
		let secret = host.host_key_secret();
		let (encoded, minted) = store
			.ensure(
				&secret,
				Some(&format!("the ssh host key of {}", host.name())),
				SecretRoll::manual(
					"delete it and its certificate, deploy, then replace the \
					 box: it reads both at first boot only",
				),
				async || {
					SshAuthority::generate_key()
						.and_then(|key| SshAuthority::encode(&key))
				},
			)
			.await?;
		if minted {
			info!("minted the ssh host key of {}", host.name());
		}
		let key = PrivateKey::from_openssh(encoded.as_bytes())?;
		let cert_secret = host.host_cert_secret();
		let held = store
			.get(&cert_secret)
			.await?
			.and_then(|cert| Certificate::from_openssh(&cert).ok())
			.filter(|cert| {
				SshAccess::certifies_host(
					cert,
					self.host.public_key(),
					key.public_key(),
					host,
				)
				.is_ok()
			});
		if held.is_some() {
			return Ok(());
		}
		let cert = SshAuthority::sign(
			&self.host,
			key.public_key(),
			CertType::Host,
			host.name(),
			&[host.name()],
			u64::MAX,
			&[],
		)?;
		store
			.overwrite(
				&cert_secret,
				&cert.to_openssh()?,
				Some(&format!(
					"the ssh host certificate of {}, signed by the stack's \
					 host authority",
					host.name()
				)),
				Some(SecretRoll::manual(
					"derived from the host key and the host authority: roll \
					 either",
				)),
			)
			.await?;
		info!("signed the ssh host certificate of {}", host.name());
		Ok(())
	}
}

/// The stack's authority as a deploy holds it to reach a box: the user
/// authority's key, which signs each login, and the host authority's public
/// half, which every connection's known hosts trust.
#[derive(Clone)]
pub struct SshAccess {
	user_ca: PrivateKey,
	host_ca: PublicKey,
}

impl std::fmt::Debug for SshAccess {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("SshAccess")
			.field("user_ca", &self.user_ca.fingerprint(HashAlg::Sha256))
			.field("host_ca", &self.host_ca.fingerprint(HashAlg::Sha256))
			.finish()
	}
}

impl SshAccess {
	/// A login certificate for `public` as `host`'s user, good for
	/// [`SshAuthority::LOGIN_VALIDITY`].
	pub fn sign_login(
		&self,
		public: &PublicKey,
		host: &SshHost,
		key_id: &str,
	) -> Result<Certificate> {
		let valid_before =
			(time_ext::now() + SshAuthority::LOGIN_VALIDITY).as_secs();
		SshAuthority::sign(
			&self.user_ca,
			public,
			CertType::User,
			key_id,
			&[host.user()],
			valid_before,
			&SshAuthority::LOGIN_EXTENSIONS,
		)
	}

	/// The one `known_hosts` line a connection to `host` trusts: its name,
	/// certified by the stack's host authority, and no key of its own.
	pub fn known_hosts(&self, host: &SshHost) -> Result<String> {
		format!(
			"@cert-authority {} {}\n",
			host.name(),
			self.host_ca.to_openssh()?
		)
		.xok()
	}

	/// Whether `cert` is `host`'s certificate for `public`, signed by `ca`
	/// and valid now, which is what the client's `@cert-authority` check
	/// asks of the box at every login. An error saying which part fails.
	fn certifies_host(
		cert: &Certificate,
		ca: &PublicKey,
		public: &PublicKey,
		host: &SshHost,
	) -> Result {
		cert.validate([&ca.fingerprint(HashAlg::Sha256)])
			.map_err(|_| {
				bevyhow!(
					"the host certificate of {} is not signed by the stack's \
					 host authority, or is not valid now",
					host.name()
				)
			})?;
		if cert.cert_type() != CertType::Host {
			bevybail!(
				"the host certificate of {} is a user certificate",
				host.name()
			);
		}
		if cert.public_key() != public.key_data() {
			bevybail!(
				"the host certificate of {} certifies another key",
				host.name()
			);
		}
		if cert.valid_principals() != [host.name().to_string()] {
			bevybail!(
				"the host certificate of {} names {:?}",
				host.name(),
				cert.valid_principals()
			);
		}
		Ok(())
	}
}

/// What [`EnsureSshAuthority`] and [`SshShell`] read: the stack's secret
/// store and every box under the stack that trusts its authority.
#[derive(SystemParam)]
pub(crate) struct SshHostQuery<'w, 's> {
	stacks: StackQuery<'w, 's>,
	#[cfg(feature = "mail")]
	mail_boxes: Query<'w, 's, &'static StalwartBlock>,
	#[cfg(feature = "lightsail_block")]
	lightsail_boxes: Query<'w, 's, &'static LightsailBlock>,
}

impl SshHostQuery<'_, '_> {
	/// The store and the hosts under `entity`'s stack, refusing a stack with
	/// none: the step would mint an authority nothing trusts.
	pub(crate) fn resolve(
		&self,
		entity: Entity,
	) -> Result<(SecretStore, Vec<SshHost>)> {
		let store = self.stacks.secret_store(entity)?;
		let (_, stack) = self.stacks.root(entity)?;
		#[cfg_attr(
			not(any(feature = "mail", feature = "lightsail_block")),
			allow(unused_variables, unused_mut)
		)]
		let (declared, mut hosts) = (self.stacks.declared(entity)?, Vec::new());
		#[cfg(feature = "mail")]
		hosts.extend(
			declared
				.iter()
				.filter_map(|child| self.mail_boxes.get(*child).ok())
				.map(StalwartBlock::ssh_host),
		);
		#[cfg(feature = "lightsail_block")]
		hosts.extend(
			declared
				.iter()
				.filter_map(|child| self.lightsail_boxes.get(*child).ok())
				.map(|block| block.ssh_host(&stack)),
		);
		if hosts.is_empty() {
			bevybail!(
				"<EnsureSshAuthority/> found no box under the stack `{}--{}` \
				 that trusts an ssh authority: declare one, or remove the step",
				stack.app_name(),
				stack.stage()
			);
		}
		(store, hosts).xok()
	}

	/// The store and the one host labelled `label` under `entity`'s stack,
	/// or its only host when `label` is `None`.
	pub(crate) fn resolve_one(
		&self,
		entity: Entity,
		label: Option<&str>,
	) -> Result<(SecretStore, SshHost)> {
		let (store, hosts) = self.resolve(entity)?;
		let labels = || {
			hosts
				.iter()
				.map(|host| host.label().to_string())
				.collect::<Vec<_>>()
				.join(", ")
		};
		let host = match label {
			Some(label) => hosts
				.iter()
				.find(|host| host.label() == label)
				.ok_or_else(|| {
					bevyhow!(
						"no box labelled `{label}` here; declared: {}",
						labels()
					)
				})?,
			None if hosts.len() == 1 => &hosts[0],
			None => bevybail!(
				"this stack declares several boxes, so name one with \
				 `--label`: {}",
				labels()
			),
		};
		(store, host.clone()).xok()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::types::test_support::*;

	/// A stack with a document store and a mail-shaped host.
	fn store() -> (World, SecretStore) {
		let mut world = infra_world();
		let root = world
			.spawn((
				Stack::new("app").with_stage("dev"),
				BlobStore::temp(),
				RepoStore,
				DocumentSecrets::new("secrets.toml"),
			))
			.id();
		world.flush();
		let store = world.get::<SecretStore>(root).unwrap().clone();
		(world, store)
	}

	fn host() -> SshHost {
		SshHost::new(
			"mail",
			"mail.example.com",
			SshAuthority::EC2_USER,
			"mail_public_ip",
		)
	}

	/// The authority mints once: a second ensure reads the same keys back,
	/// and the public halves sit beside them.
	#[beet_core::test]
	async fn mints_the_authority_once() {
		let (_world, store) = store();
		let first = SshAuthority::ensure(&store).await.unwrap();
		let second = SshAuthority::ensure(&store).await.unwrap();
		first
			.user
			.fingerprint(HashAlg::Sha256)
			.xpect_eq(second.user.fingerprint(HashAlg::Sha256));
		store
			.get(&SshAuthority::host_ca_public())
			.await
			.unwrap()
			.unwrap()
			.xpect_eq(first.host.public_key().to_openssh().unwrap());
	}

	/// A login is a user certificate from the user authority, for the box's
	/// account alone, valid for minutes, permitting the tunnel.
	#[beet_core::test]
	async fn signs_a_login_for_minutes() {
		let (_world, store) = store();
		let keys = SshAuthority::ensure(&store).await.unwrap();
		let access = SshAuthority::access(&store).await.unwrap();
		let login = SshAuthority::generate_key().unwrap();
		let cert = access
			.sign_login(login.public_key(), &host(), "app--dev deploy")
			.unwrap();
		cert.validate([&keys.user.fingerprint(HashAlg::Sha256)])
			.unwrap();
		cert.cert_type().xpect_eq(CertType::User);
		cert.valid_principals().xpect_eq(["ec2-user".to_string()]);
		(cert.valid_before() - cert.valid_after()).xpect_eq(
			(SshAuthority::LOGIN_VALIDITY + SshAuthority::CLOCK_SKEW).as_secs(),
		);
		cert.extensions()
			.contains_key("permit-port-forwarding")
			.xpect_true();
		cert.extensions()
			.contains_key("permit-agent-forwarding")
			.xpect_false();
		// the host authority signs hosts, never logins
		cert.validate([&keys.host.fingerprint(HashAlg::Sha256)])
			.unwrap_err();
	}

	/// The host certificate is signed once and kept while it holds, and
	/// signed again when the box's name changes.
	#[beet_core::test]
	async fn signs_the_host_until_it_holds() {
		let (_world, store) = store();
		let keys = SshAuthority::ensure(&store).await.unwrap();
		keys.ensure_host(&store, &host()).await.unwrap();
		let read = async |store: &SecretStore| {
			store
				.get(&host().host_cert_secret())
				.await
				.unwrap()
				.unwrap()
		};
		let first = read(&store).await;
		keys.ensure_host(&store, &host()).await.unwrap();
		read(&store).await.xpect_eq(first.clone());

		let renamed = SshHost::new(
			"mail",
			"mx.example.com",
			SshAuthority::EC2_USER,
			"mail_public_ip",
		);
		keys.ensure_host(&store, &renamed).await.unwrap();
		let cert = Certificate::from_openssh(&read(&store).await).unwrap();
		cert.valid_principals()
			.xpect_eq(["mx.example.com".to_string()]);
		let key = store
			.get(&host().host_key_secret())
			.await
			.unwrap()
			.unwrap()
			.xmap(|key| PrivateKey::from_openssh(key.as_bytes()))
			.unwrap();
		SshAccess::certifies_host(
			&cert,
			keys.host.public_key(),
			key.public_key(),
			&renamed,
		)
		.unwrap();
	}

	/// A certificate from any other authority, for any other key or any
	/// other name, does not certify the host.
	#[beet_core::test]
	fn refuses_a_certificate_that_does_not_hold() {
		let ca = SshAuthority::generate_key().unwrap();
		let other_ca = SshAuthority::generate_key().unwrap();
		let key = SshAuthority::generate_key().unwrap();
		let sign = |ca: &PrivateKey, key: &PrivateKey, name: &str| {
			SshAuthority::sign(
				ca,
				key.public_key(),
				CertType::Host,
				name,
				&[name],
				u64::MAX,
				&[],
			)
			.unwrap()
		};
		let holds = |cert: &Certificate| {
			SshAccess::certifies_host(
				cert,
				ca.public_key(),
				key.public_key(),
				&host(),
			)
		};
		holds(&sign(&ca, &key, "mail.example.com")).unwrap();
		holds(&sign(&other_ca, &key, "mail.example.com"))
			.unwrap_err()
			.to_string()
			.xpect_contains("not signed by the stack's host authority");
		holds(&sign(
			&ca,
			&SshAuthority::generate_key().unwrap(),
			"mail.example.com",
		))
		.unwrap_err()
		.to_string()
		.xpect_contains("certifies another key");
		holds(&sign(&ca, &key, "mx.example.com"))
			.unwrap_err()
			.to_string()
			.xpect_contains("names");
	}

	/// A connection's known hosts trust the host authority for the box's
	/// name and nothing else.
	#[beet_core::test]
	async fn known_hosts_trust_the_authority_by_name() {
		let (_world, store) = store();
		let keys = SshAuthority::ensure(&store).await.unwrap();
		SshAuthority::access(&store)
			.await
			.unwrap()
			.known_hosts(&host())
			.unwrap()
			.xpect_eq(format!(
				"@cert-authority mail.example.com {}\n",
				keys.host.public_key().to_openssh().unwrap()
			));
	}

	/// A connection to a stack that never minted its authority names the
	/// step that mints it.
	#[beet_core::test]
	async fn an_unminted_authority_names_its_step() {
		let (_world, store) = store();
		SshAuthority::access(&store)
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("<EnsureSshAuthority/>");
	}
}
