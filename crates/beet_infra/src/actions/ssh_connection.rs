//! The management channel to a deployed box: the deploy reaches it to roll a
//! running instance onto a new release ([`LightsailRelease`]) and to provision
//! a mail server ([`StalwartProvision`]), and the same connection serves an
//! operator's session on a box that is misbehaving.
use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use ssh_key::PrivateKey;
use std::process::ExitStatus;
use std::process::Output;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

/// `<SshShell/>` — a session on one of the stack's boxes, as the deploy
/// reaches it: logged in with a certificate the stack's [`SshAuthority`]
/// signs for this session alone, the box verified by its host certificate.
/// No key on this machine is involved, so any machine that can read the
/// stack's secrets can reach its boxes, and no other can.
///
/// A shell, or one command whose output is this verb's answer:
///
/// ```sh
/// beet mail/ssh --stage=prod
/// beet mail/ssh --stage=prod --command='sudo tail /var/log/stalwart/stalwart.log'
/// ```
#[action]
#[derive(Debug, Component, Reflect)]
#[reflect(Component, Default)]
pub async fn SshShell(
	/// The box, by its block's label, ie `mail`: needed only where the stack
	/// declares several.
	#[param]
	label: Option<String>,
	/// A command to run instead of a shell.
	#[param]
	command: Option<String>,
	cx: ActionContext<Request>,
) -> Result<Response> {
	let backend = terra::Project::resolve_backend(&cx.caller).await?;
	let (project, store, host) = cx
		.caller
		.with_world(move |world, entity| -> Result<_> {
			let project = terra::Project::resolve_in(world, entity, backend)?;
			let (store, host) =
				world.with_state::<SshHostQuery, _>(|query| {
					query.resolve_one(entity, label.as_deref())
				})?;
			(project, store, host).xok()
		})
		.await??;
	let name = host.name().clone();
	let status = SshConnection::to(&project, &store, host)
		.await?
		.session(command.as_deref())
		.await?;
	match status.success() {
		true => Response::ok().xok(),
		false => bevybail!("the ssh session on {name} ended with {status}"),
	}
}

/// An ssh connection to one box under its stack's [`SshAuthority`].
///
/// Nothing on this machine is a credential. The connection generates a key of
/// its own, held in a private temporary directory for its lifetime, and logs
/// in with a certificate the stack's user authority signed for minutes,
/// signing again before a login once the current one is half spent. The box
/// is verified the same way in reverse: the only `known_hosts` line is the
/// host authority, trusted for the box's [`name`](SshHost::name), so a box
/// whose host certificate the authority did not sign is refused rather than
/// trusted on first use.
#[derive(Debug, Clone)]
pub struct SshConnection {
	/// The address dialled, ie the box's public ip.
	address: String,
	/// The box: the name its host certificate names, its login and its port.
	host: SshHost,
	/// This connection's key, its logins and its known hosts.
	identity: Arc<SshIdentity>,
}

impl SshConnection {
	/// A connection to `host` at the address its stack's applied `project`
	/// outputs, signing its logins with the authority in `store`.
	pub async fn to(
		project: &terra::Project,
		store: &SecretStore,
		host: SshHost,
	) -> Result<Self> {
		let address = project.output(host.address_output()).await?;
		Self::new(SshAuthority::access(store).await?, host, address)
	}

	/// A connection to `host` at `address` with `access` already read.
	pub fn new(
		access: SshAccess,
		host: SshHost,
		address: impl Into<String>,
	) -> Result<Self> {
		Self {
			identity: SshIdentity::new(access, &host)?.xmap(Arc::new),
			address: address.into(),
			host,
		}
		.xok()
	}

	/// The address dialled.
	pub fn address(&self) -> &str { &self.address }

	/// The `user@address` string for ssh commands.
	pub fn remote_user(&self) -> String {
		format!("{}@{}", self.host.user(), self.address)
	}

	/// Every option a connection runs with: the box verified through the
	/// host authority alone, this connection's key and a current login
	/// certificate as the only identity (no agent, no `~/.ssh`), no prompt,
	/// and a keepalive, since a release can sit near-silent for minutes while
	/// a fresh box crawls through cloud-init and a dropped session should
	/// fail fast rather than hang the deploy. The port is not among them,
	/// since `ssh` spells it `-p` and `scp` `-P`.
	fn ssh_args(&self) -> Result<Vec<SmolStr>> {
		let identity = &self.identity;
		let login = identity.login(&self.host)?;
		[
			("StrictHostKeyChecking", "yes".to_string()),
			(
				"UserKnownHostsFile",
				identity.known_hosts_path().to_string(),
			),
			("GlobalKnownHostsFile", "/dev/null".to_string()),
			("HostKeyAlias", self.host.name().to_string()),
			("CheckHostIP", "no".to_string()),
			("UpdateHostKeys", "no".to_string()),
			("IdentitiesOnly", "yes".to_string()),
			("IdentityAgent", "none".to_string()),
			("IdentityFile", identity.key_path().to_string()),
			("CertificateFile", login.to_string()),
			("ConnectTimeout", "30".to_string()),
			("BatchMode", "yes".to_string()),
			("ServerAliveInterval", "15".to_string()),
		]
		.into_iter()
		.flat_map(|(key, value)| {
			[SmolStr::new("-o"), SmolStr::new(format!("{key}={value}"))]
		})
		.collect::<Vec<_>>()
		.xok()
	}

	/// `ssh -p <port> user@address`, every option before it.
	fn ssh_command(&self) -> Result<Vec<SmolStr>> {
		let mut args = self.ssh_args()?;
		args.push("-p".into());
		args.push(self.host.port().to_string().into());
		args.push(self.remote_user().into());
		args.xok()
	}

	/// Run a command on the box, returning its output. A non-zero exit is an
	/// error carrying the remote stderr.
	pub async fn run_command(&self, command: &str) -> Result<Output> {
		let mut args = self.ssh_command()?;
		args.push(command.into());
		ChildProcess::new("ssh").with_args(args).run_async().await
	}

	/// An operator's session on the box, sharing this terminal: a shell, or
	/// `command` when given, answering its exit status.
	pub async fn session(&self, command: Option<&str>) -> Result<ExitStatus> {
		let mut args = self.ssh_command()?;
		args.extend(command.map(SmolStr::new));
		ChildProcess::new("ssh")
			.with_args(args)
			.spawn()?
			.status()
			.await
	}

	/// Copy a local file to the box via scp.
	pub async fn scp_to(
		&self,
		local_path: &AbsPath,
		remote_path: &str,
	) -> Result {
		let mut args = self.ssh_args()?;
		args.push("-P".into());
		args.push(self.host.port().to_string().into());
		args.push(local_path.to_string().into());
		args.push(format!("{}:{}", self.remote_user(), remote_path).into());
		ChildProcess::new("scp").with_args(args).run_async().await?;
		Ok(())
	}

	/// Forward `local_port` on this machine to `remote_port` on the box's
	/// loopback, for a service the security group deliberately does not admit
	/// from the internet (the mail box's management endpoint on 8080).
	///
	/// The returned handle kills the forward on drop, so a caller cannot leave
	/// a port open past the step that needed it. `-N` runs no remote command,
	/// and `ExitOnForwardFailure` turns a port already in use into an immediate
	/// error rather than a live ssh session forwarding nothing.
	pub async fn tunnel(
		&self,
		local_port: u16,
		remote_port: u16,
	) -> Result<ChildHandle> {
		let mut args = self.ssh_args()?;
		args.push("-o".into());
		args.push("ExitOnForwardFailure=yes".into());
		args.push("-N".into());
		args.push("-p".into());
		args.push(self.host.port().to_string().into());
		args.push("-L".into());
		args.push(
			format!("{local_port}:127.0.0.1:{remote_port}")
				.as_str()
				.into(),
		);
		args.push(self.remote_user().into());
		let handle = ChildProcess::new("ssh").with_args(args).spawn()?;
		info!(
			"tunnelling localhost:{local_port} to {}:{remote_port}",
			self.address
		);
		Ok(handle)
	}

	/// Wait for ssh to answer, retrying every `poll` until `timeout` elapses.
	/// At least one attempt is always made, and the final failure carries
	/// the last attempt's error rather than only a count.
	///
	/// A fresh box answers before cloud-init has installed its host
	/// certificate, which reads here as `Host key verification failed` until
	/// it has: the wait is what covers that, never a weaker check.
	pub async fn wait_for_ready(
		&self,
		timeout: Duration,
		poll: Duration,
	) -> Result {
		let max_attempts = (timeout.as_secs() / poll.as_secs().max(1)).max(1);
		let mut last_err = None;
		for attempt in 1..=max_attempts {
			info!(
				"waiting for ssh on {}:{} (attempt {attempt}/{max_attempts})...",
				self.address,
				self.host.port()
			);
			match self.run_command("echo ready").await {
				Ok(_) => return Ok(()),
				Err(err) => last_err = Some(err),
			}
			time_ext::sleep(poll).await;
		}
		bevybail!(
			"failed to connect to {} at {}:{} after {max_attempts} attempts, \
			 the last with: {}",
			self.host.name(),
			self.address,
			self.host.port(),
			last_err.map(|err| err.to_string()).unwrap_or_default()
		)
	}
}

/// What a connection logs in with: its own key, generated for it and never
/// stored, the login certificates signed for that key, and the known hosts
/// trusting the box's authority, all in a private directory removed with it.
struct SshIdentity {
	access: SshAccess,
	key: PrivateKey,
	dir: TempDir,
	/// The current login certificate and when it was signed.
	login: Mutex<Option<(AbsPath, Duration)>>,
	/// How many logins have been signed, so each lands in its own file and a
	/// running ssh never reads one mid-write.
	signed: AtomicUsize,
}

impl std::fmt::Debug for SshIdentity {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("SshIdentity")
			.field("access", &self.access)
			.field("dir", &self.dir.path())
			.finish_non_exhaustive()
	}
}

impl SshIdentity {
	/// A fresh key and the known hosts for `host`, written owner-only.
	fn new(access: SshAccess, host: &SshHost) -> Result<Self> {
		let identity = Self {
			key: SshAuthority::generate_key()?,
			dir: TempDir::new()?,
			login: default(),
			signed: default(),
			access,
		};
		fs_ext::write_private(
			identity.key_path(),
			SshAuthority::encode(&identity.key)?,
		)?;
		fs_ext::write(
			identity.known_hosts_path(),
			identity.access.known_hosts(host)?,
		)?;
		identity.xok()
	}

	fn key_path(&self) -> AbsPath { self.dir.join("id_ed25519") }

	fn known_hosts_path(&self) -> AbsPath { self.dir.join("known_hosts") }

	/// A login certificate with at least half its validity left, signed now
	/// if the current one is past that.
	fn login(&self, host: &SshHost) -> Result<AbsPath> {
		let mut login = self.login.lock().map_err(|_| {
			bevyhow!("an ssh login was signed on a panicked thread")
		})?;
		let now = time_ext::now();
		if let Some((path, _)) = login.as_ref().filter(|(_, signed)| {
			now < *signed + SshAuthority::LOGIN_VALIDITY / 2
		}) {
			return path.clone().xok();
		}
		let key_id = format!(
			"beet deploy by {} to {}",
			env_ext::var("USER").unwrap_or_else(|_| "unknown".into()),
			host.name()
		);
		let cert =
			self.access
				.sign_login(self.key.public_key(), host, &key_id)?;
		let count = self.signed.fetch_add(1, Ordering::Relaxed);
		let path = self.dir.join(format!("login-{count}-cert.pub"));
		fs_ext::write(&path, cert.to_openssh()?)?;
		*login = Some((path.clone(), now));
		path.xok()
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use crate::types::test_support::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;

	/// A stack's store, its authority minted and `host`'s identity signed.
	async fn authority(host: &SshHost) -> (World, SecretStore) {
		let mut world = infra_world();
		let root = world
			.spawn((
				Stack::new("app").with_stage("dev"),
				BlobStore::temp(),
				PackageStore,
				DocumentSecrets::new(fwd_path!("secrets.toml")),
			))
			.id();
		world.flush();
		let store = world.get::<SecretStore>(root).unwrap().clone();
		SshAuthority::ensure(&store)
			.await
			.unwrap()
			.ensure_host(&store, host)
			.await
			.unwrap();
		(world, store)
	}

	/// An unprivileged sshd on loopback serving `host`'s identity from
	/// `store`, trusting the user authority in `trusted`: the box's own trust
	/// stanza, run as this user. `None` when this machine has no sshd.
	async fn serve(
		host: &SshHost,
		store: &SecretStore,
		trusted: &SecretStore,
	) -> Option<(TempDir, ChildHandle, u16)> {
		let sshd = ["/usr/sbin/sshd", "/usr/bin/sshd"]
			.into_iter()
			.find(|path| fs_ext::exists(path).unwrap_or(false))?;
		let dir = TempDir::new().unwrap();
		let read = async |store: &SecretStore, secret: &SecretRef| {
			store.get(secret).await.unwrap().unwrap()
		};
		let key = dir.join("host_key");
		fs_ext::write_private(&key, read(store, &host.host_key_secret()).await)
			.unwrap();
		fs_ext::write(
			dir.join("host_key-cert.pub"),
			read(store, &host.host_cert_secret()).await,
		)
		.unwrap();
		fs_ext::write(
			dir.join("user_ca.pub"),
			read(trusted, &SshAuthority::user_ca_public()).await,
		)
		.unwrap();
		let port = std::net::TcpListener::bind("127.0.0.1:0")
			.unwrap()
			.local_addr()
			.unwrap()
			.port();
		fs_ext::write(
			dir.join("sshd_config"),
			format!(
				"ListenAddress 127.0.0.1\nPort {port}\nHostKey {key}\n\
				 HostCertificate {key}-cert.pub\nTrustedUserCAKeys {dir}/user_ca.pub\n\
				 AuthorizedKeysFile none\nPidFile none\nStrictModes no\n\
				 UsePAM no\nLogLevel ERROR\n",
				dir = dir.path()
			),
		)
		.unwrap();
		let handle = ChildProcess::new(sshd)
			.with_args([
				"-D",
				"-f",
				dir.join("sshd_config").to_string().as_str(),
			])
			.spawn_piped()
			.unwrap();
		// listening before any connection, so a refusal is the host's
		// certificate and never a race with the daemon's start
		for _ in 0..100 {
			if std::net::TcpStream::connect(("127.0.0.1", port)).is_ok() {
				return Some((dir, handle, port));
			}
			time_ext::sleep(Duration::from_millis(20)).await;
		}
		panic!("sshd never listened on {port}");
	}

	/// This user, as the login an unprivileged sshd can admit.
	fn host(name: &str) -> SshHost {
		SshHost::new("mail", name, env_ext::var("USER").unwrap(), "unused")
	}

	/// A connection to `host` served on loopback at `port`.
	async fn connect(
		store: &SecretStore,
		host: &SshHost,
		port: u16,
	) -> SshConnection {
		SshConnection::new(
			SshAuthority::access(store).await.unwrap(),
			host.clone().with_port(port),
			"127.0.0.1",
		)
		.unwrap()
	}

	/// The whole trust, against a real sshd: a login certificate the user
	/// authority signed gets in, and the box's host certificate verifies
	/// with nothing pinned but the host authority.
	#[beet_core::test]
	async fn logs_in_and_verifies_the_host() {
		let host = host("mail.example.com");
		let (_world, store) = authority(&host).await;
		let Some((_dir, _sshd, port)) = serve(&host, &store, &store).await
		else {
			warn!("no sshd on this machine, so the live ssh trust is untested");
			return;
		};
		connect(&store, &host, port)
			.await
			.run_command("echo certified")
			.await
			.unwrap()
			.stdout
			.xmap(|stdout| String::from_utf8(stdout).unwrap())
			.xpect_eq("certified\n");
	}

	/// A box whose host certificate another authority signed is refused, as
	/// is one certified under another name: no first use is trusted.
	#[beet_core::test]
	async fn refuses_a_host_it_cannot_verify() {
		let host = host("mail.example.com");
		let (_world, store) = authority(&host).await;
		let (_other_world, other) = authority(&host).await;
		let Some((_dir, _sshd, port)) = serve(&host, &other, &store).await
		else {
			warn!("no sshd on this machine, so the live ssh trust is untested");
			return;
		};
		connect(&store, &host, port)
			.await
			.run_command("echo trusted")
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("Host key verification failed");

		// the right authority, but certified for another name
		let Some((_dir, _sshd, port)) = serve(&host, &store, &store).await
		else {
			return;
		};
		connect(&store, &self::host("mx.example.com"), port)
			.await
			.run_command("echo trusted")
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("Host key verification failed");
	}
}
