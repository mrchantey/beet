//! What keeps a small box serving through a memory spike, and leaves a reading
//! behind either way.
use beet_core::prelude::*;

/// The machine-level memory config every box block renders: a swapfile, a
/// ceiling on the unit that matters, and a per-minute sampler.
///
/// A 2 GB box with no swap has no headroom and no graceful degradation: under
/// pressure the kernel's only move is to kill the largest process, which is
/// always the app, and nothing records why. That is how `beet.org` went down on
/// 2026-09-30, and its box was not special — every box block beet deploys to
/// lands on hardware of the same size with the same defaults.
///
/// Three pieces, which are the three different failures:
///
/// - a **swapfile**, so a spike degrades into slowness rather than a kill;
/// - **`MemoryHigh` / `MemoryMax` / `MemorySwapMax`** on the unit, so an overrun
///   reclaims, spills into a bounded slice of swap, and finally takes THAT unit
///   alone instead of the kernel choosing a victim and possibly taking the TLS
///   terminator or the management ssh daemon with it;
/// - a **sampler**, so the next occurrence is a curve rather than the single
///   number the kernel prints on its way out.
///
/// All of it is machine config, so it rides the box's `user_data` and a change
/// to it replaces the box. That is the right trade: a hand-installed drop-in is
/// lost at the next rebuild, which is exactly when it is most wanted.
///
/// ## Why it is all best-effort
///
/// `user_data` runs under `set -e`. A guard against a spike that aborts the boot
/// is strictly worse than the spike, so every piece below is independently
/// fail-safe and [`setup_script`](Self::setup_script) backstops whatever they
/// miss: the worst case is a box with no swap and no sampler rather than no box.
///
/// It is also self-reporting, which is what makes best-effort safe rather than
/// silent. A box whose swapfile failed carries `swapfree_kb=0` on every sample,
/// and a box whose sampler failed has no samples at all, so either failure shows
/// up in the one place a reader is already looking.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryGuards {
	/// What the sampler's unit, script, log and sysctl drop-in are named after,
	/// and the unit the ceiling applies to: `beet-site` yields
	/// `beet-site-mem.timer` sampling `beet-site.service`.
	name: SmolStr,
	/// The swapfile's size in MiB.
	swapfile_mb: u64,
	/// The unit's ceiling, absent on a box too small to carry one that is not
	/// hostile (see [`for_ram_mb`](Self::for_ram_mb)).
	ceiling: Option<Ceiling>,
	/// A `KEY=value` file holding `BEET_DEPLOY_ID`, so every sample names the
	/// artifact it belongs to. `None` on a box running something that is not a
	/// beet release, which simply omits the field.
	release_env: Option<SmolStr>,
}

/// A unit's memory ceiling, in MiB.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Ceiling {
	/// `MemoryHigh`: where the kernel starts reclaiming.
	high_mb: u64,
	/// `MemoryMax`: the wall.
	max_mb: u64,
	/// `MemorySwapMax`: how far past the wall the unit may spill before it is
	/// killed. Bounded so a runaway dies in seconds instead of thrashing the
	/// box indefinitely.
	swap_max_mb: u64,
}

impl MemoryGuards {
	/// Left for the kernel, the TLS terminator, the log agent and the
	/// management ssh daemon, under [`for_ram_mb`](Self::for_ram_mb).
	const HEADROOM_MB: u64 = 600;
	/// The gap between `MemoryHigh` and `MemoryMax`: room to reclaim in before
	/// hitting the wall.
	const RECLAIM_MB: u64 = 400;
	/// The smallest ceiling worth imposing. Below this a cap is not a guard but
	/// a second way to die: `beet.org` idles at ~156 MB and peaks around 560 MB
	/// under its own request caps, so a unit capped under half a gig would be
	/// killed by its own ordinary traffic.
	const MIN_CEILING_MB: u64 = 512;

	/// Guards sized for a box with `ram_mb` of USABLE memory, ie what the
	/// instance reports rather than what the bundle advertises: a ceiling sized
	/// against the marketing figure is a ceiling above what the box has.
	///
	/// `MemoryMax` leaves [`HEADROOM_MB`](Self::HEADROOM_MB) for everything that
	/// is not the app, `MemoryHigh` sits [`RECLAIM_MB`](Self::RECLAIM_MB) under
	/// it, and both round down to a round hundred so a unit file reads like a
	/// decision rather than an arithmetic result. The swapfile rounds the box's
	/// own size up to a whole GiB.
	///
	/// A box too small to carry that headroom and still leave
	/// [`MIN_CEILING_MB`](Self::MIN_CEILING_MB) gets **no ceiling at all**,
	/// deliberately: clamping one into the remaining space would hand the unit a
	/// cap below its own working set, which turns a guard against a rare spike
	/// into a kill on ordinary traffic. Such a box keeps the swapfile and the
	/// sampler, which cost it nothing and are what it actually needs.
	pub fn for_ram_mb(name: impl Into<SmolStr>, ram_mb: u64) -> Self {
		let round_down = |mb: u64| (mb / 100) * 100;
		let max_mb = round_down(ram_mb.saturating_sub(Self::HEADROOM_MB));
		Self {
			name: name.into(),
			swapfile_mb: ram_mb.div_ceil(1024) * 1024,
			ceiling: (max_mb >= Self::MIN_CEILING_MB).then(|| Ceiling {
				high_mb: round_down(max_mb.saturating_sub(Self::RECLAIM_MB)),
				max_mb,
				swap_max_mb: 512,
			}),
			release_env: None,
		}
	}

	/// Name the file whose `BEET_DEPLOY_ID` every sample carries, so a reading
	/// is attributable to the artifact that produced it (a later deploy
	/// invalidates the curve before it).
	pub fn with_release_env(mut self, path: impl Into<SmolStr>) -> Self {
		self.release_env = Some(path.into());
		self
	}

	/// The `[Service]` lines capping the unit, spliced into its unit file.
	///
	/// Unsupported directives are a warning rather than a failed unit on every
	/// systemd that has shipped since 2016, so this needs no version guard.
	pub fn unit_lines(&self) -> String {
		let Some(ceiling) = self.ceiling else {
			return String::new();
		};
		format!(
			"# this unit's own ceiling (see the block): reclaim at High, swap a little past\n\
			# Max, cgroup kill once SwapMax is gone too\n\
			MemoryHigh={}M\n\
			MemoryMax={}M\n\
			MemorySwapMax={}M\n",
			ceiling.high_mb, ceiling.max_mb, ceiling.swap_max_mb
		)
	}

	/// The sampler installed at `/usr/local/bin/<name>-mem`: one line per
	/// minute of the unit's resident set, its peak, what the box has left, and
	/// (where [`with_release_env`](Self::with_release_env) names one) the
	/// release it is running.
	///
	/// A creep and a spike look nothing alike at one-minute resolution and
	/// identical in hindsight without it, which is why the `beet.org` kill took
	/// a reproduction rather than a reading. `/var/log` is append-only across
	/// restarts, so the minutes BEFORE a kill survive it.
	pub fn sampler_script(&self) -> String {
		let name = &self.name;
		// the leak is always on a specific artifact, so a box that HAS a release
		// pointer names its own on every line
		let (read_release, release_field) = match &self.release_env {
			Some(path) => (
				format!(
					"# the leak is always on a specific artifact, so every sample names its own\n\
					deploy=$(sed -n 's/^BEET_DEPLOY_ID=//p' {path} 2>/dev/null | tail -1)\n"
				),
				" deploy=${deploy:-unknown}",
			),
			None => (String::new(), ""),
		};
		format!(
			r#"#!/bin/bash
# Sample the unit's memory. One line, parseable, timestamped like the app log.
set -uo pipefail
pid=$(systemctl show {name} -p MainPID --value)
# not running: the next sample says so rather than this one lying
if [ "${{pid:-0}}" = 0 ] || [ ! -r "/proc/$pid/status" ]; then exit 0; fi
rss=$(sed -n 's/^VmRSS:[[:space:]]*\([0-9]*\).*/\1/p' "/proc/$pid/status")
hwm=$(sed -n 's/^VmHWM:[[:space:]]*\([0-9]*\).*/\1/p' "/proc/$pid/status")
avail=$(awk '/^MemAvailable:/ {{print $2}}' /proc/meminfo)
swap=$(awk '/^SwapFree:/ {{print $2}}' /proc/meminfo)
restarts=$(systemctl show {name} -p NRestarts --value)
{read_release}echo "$(date -u +%Y-%m-%dT%H:%M:%S.000Z) mem rss_kb=${{rss:-0}} hwm_kb=${{hwm:-0}} avail_kb=${{avail:-0}} swapfree_kb=${{swap:-0}} restarts=${{restarts:-0}}{release_field}"
"#
		)
		.trim_end()
		.to_string()
	}

	/// The `user_data` section installing all three pieces, to be run BEFORE the
	/// unit it caps so the app has swap and a sampler from its first second.
	///
	/// Every piece is independently fail-safe and the whole is wrapped besides
	/// (see the type docs). Note `set -e` does NOT apply inside a function on
	/// the left of `||`, which is why the chains here are explicit rather than
	/// a plain sequence.
	pub fn setup_script(&self) -> String {
		let Self {
			name, swapfile_mb, ..
		} = self;
		let sampler = self.sampler_script();
		format!(
			r#"
# Best-effort, and deliberately so: cloud-init runs under `set -e`, and a guard
# against a spike must never itself be the thing that stops the box serving. Each
# piece below is independently fail-safe, and `setup_memory` is the backstop for
# anything they miss, so the worst case is a box with no swap and no sampler
# rather than no box. Note `set -e` does NOT apply inside a function on the left
# of `||`, which is exactly why the chains here are explicit.
setup_memory() {{
# a swapfile, so a spike degrades into slowness rather than a kill. A partial
# file is removed rather than left for `swapon` to choke on at the next boot.
if [ ! -f /swapfile ]; then
  dd if=/dev/zero of=/swapfile bs=1M count={swapfile_mb} status=none \
    && chmod 600 /swapfile \
    && mkswap /swapfile >/dev/null \
    && swapon /swapfile \
    && echo '/swapfile none swap sw 0 0' >> /etc/fstab \
    || {{ rm -f /swapfile; echo "beet: no swapfile, continuing" >&2; }}
fi
# a safety net, not a routine path: the app should be resident
echo 'vm.swappiness=10' > /etc/sysctl.d/90-{name}-swappiness.conf \
  && sysctl -p /etc/sysctl.d/90-{name}-swappiness.conf >/dev/null \
  || echo "beet: could not set vm.swappiness, continuing" >&2

# the per-minute memory sampler and its timer
cat > /usr/local/bin/{name}-mem <<'MEM_EOF'
{sampler}
MEM_EOF
chmod +x /usr/local/bin/{name}-mem

cat > /etc/systemd/system/{name}-mem.service <<'MEM_UNIT_EOF'
[Unit]
Description=Sample {name}'s memory

[Service]
Type=oneshot
ExecStart=/usr/local/bin/{name}-mem
StandardOutput=append:/var/log/{name}-mem.log
StandardError=append:/var/log/{name}-mem.log
MEM_UNIT_EOF

cat > /etc/systemd/system/{name}-mem.timer <<'MEM_TIMER_EOF'
[Unit]
Description=Sample {name}'s memory every minute

[Timer]
OnBootSec=1min
OnUnitActiveSec=1min

[Install]
WantedBy=timers.target
MEM_TIMER_EOF
systemctl daemon-reload \
  && systemctl enable --now {name}-mem.timer \
  || echo "beet: memory sampler timer not started, continuing" >&2
}}
setup_memory || echo "beet: memory guards and sampler not installed, continuing" >&2
"#
		)
	}

	/// The log the sampler appends to, for a log-forwarding agent to collect.
	pub fn log_path(&self) -> String {
		format!("/var/log/{}-mem.log", self.name)
	}

	/// The log stream the samples belong in, beside the app's own.
	pub fn log_stream(&self) -> String { format!("{}-mem", self.name) }
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// The derivation lands on the figures prod runs: a 1913 MB box (what a
	/// Lightsail `small_3_0` actually reports) caps at 1300M with 900M of
	/// reclaim under it and a 2 GiB swapfile.
	#[beet_core::test]
	fn a_two_gig_box_is_sized_as_prod_is() {
		let guards = MemoryGuards::for_ram_mb("app", 1913);
		guards
			.unit_lines()
			.as_str()
			.xpect_contains("MemoryHigh=900M")
			.xpect_contains("MemoryMax=1300M")
			.xpect_contains("MemorySwapMax=512M");
		guards.setup_script().xpect_contains("count=2048");
	}

	/// A box too small to carry the headroom gets NO ceiling, rather than one
	/// clamped below its own working set.
	///
	/// This is the difference between a guard and a second way to die: a unit
	/// capped under half a gig is killed by ordinary traffic, not by a spike.
	/// The swapfile and the sampler cost such a box nothing and still apply.
	#[beet_core::test]
	fn a_box_too_small_for_a_ceiling_gets_none() {
		for ram in [256u64, 512, 961, 1000] {
			MemoryGuards::for_ram_mb("app", ram)
				.unit_lines()
				.xpect_eq("");
		}
		// and the halves that always make sense survive
		MemoryGuards::for_ram_mb("app", 512)
			.setup_script()
			.as_str()
			.xpect_contains("mkswap /swapfile")
			.xpect_contains("app-mem.timer");
	}

	/// Every piece is fail-safe and the whole is wrapped besides, because
	/// `user_data` runs under `set -e` and a guard that aborts the boot is
	/// strictly worse than the spike it guards against.
	#[beet_core::test]
	fn nothing_in_it_can_abort_a_boot() {
		MemoryGuards::for_ram_mb("app", 1913)
			.setup_script()
			.as_str()
			// the backstop
			.xpect_contains("setup_memory || echo")
			// a partial swapfile is removed rather than left for the next boot
			.xpect_contains("rm -f /swapfile")
			// and each piece carries its own fallback
			.xpect_contains("could not set vm.swappiness, continuing")
			.xpect_contains("memory sampler timer not started, continuing");
	}

	/// A box running something that is not a beet release omits the `deploy=`
	/// field rather than carrying a permanently `unknown` one.
	#[beet_core::test]
	fn a_sample_names_its_release_only_where_there_is_one() {
		let anonymous = MemoryGuards::for_ram_mb("stalwart", 1898);
		anonymous
			.sampler_script()
			.as_str()
			.xnot()
			.xpect_contains("deploy=")
			// the rest of the line is the same wherever it is sampled
			.xpect_contains("rss_kb=")
			.xpect_contains("swapfree_kb=");
		anonymous
			.with_release_env("/etc/app/deploy.env")
			.sampler_script()
			.as_str()
			.xpect_contains("BEET_DEPLOY_ID=")
			.xpect_contains("deploy=${deploy:-unknown}");
	}
}
