//! Pure mDNS (RFC 1035 over multicast) wire helpers for service browsing.
//!
//! These are *pure functions* over byte buffers — no sockets, no world, no
//! allocation beyond the parsed output — exactly like
//! [`http_ext`](crate::types::http_ext) is for HTTP. They are `no_std` and
//! shared by the agnostic [`browser`](super::browser) engine (and reusable by a
//! downstream esp responder/resolver). The browser is the consumer: it builds a
//! `PTR` query for a service type, sends it over the UDP seam, and feeds inbound
//! datagrams back through [`parse_response`] to enumerate services.
//!
//! Unlike the A-record-only codec in the esp firmware, this parses the four
//! record types needed to *enumerate* a service:
//!
//! - `PTR` `_http._tcp.local` → `My Device._http._tcp.local` (instance name)
//! - `SRV` `<instance>` → host + port
//! - `A` `<host>` → IPv4
//! - `TXT` `<instance>` → key/value metadata (optional)
//!
//! Inbound mDNS responses pack these records and compress names aggressively, so
//! the reader follows compression pointers (`0xc0`) when decoding names. The
//! query writer emits a single uncompressed `PTR` question.

use beet_core::prelude::*;
use core::net::Ipv4Addr;

/// DNS `TYPE` for an IPv4 host address (`A`).
pub const TYPE_A: u16 = 1;
/// DNS `TYPE` for a domain-name pointer (`PTR`) — the service-instance list.
pub const TYPE_PTR: u16 = 12;
/// DNS `TYPE` for arbitrary text records (`TXT`) — service metadata.
pub const TYPE_TXT: u16 = 16;
/// DNS `TYPE` for a service location record (`SRV`) — host + port.
pub const TYPE_SRV: u16 = 33;

/// DNS `CLASS` `IN` (internet).
pub const CLASS_IN: u16 = 1;
/// Mask for the cache-flush / unicast-response bit mDNS overloads onto `CLASS`.
pub const CLASS_MASK: u16 = 0x7fff;

/// A single resource record parsed from an mDNS response.
///
/// Only the four browse-relevant record types are decoded; everything else is
/// surfaced as [`Record::Other`] so the answer count still lines up while
/// scanning. Names are owned `String`s (decompressed); the byte budget is tiny
/// (a handful of records per packet).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Record {
	/// `PTR <service_type> -> <instance>`: an instance of the queried service.
	Ptr {
		/// The queried service type, e.g. `_http._tcp.local`.
		name: String,
		/// The pointed-to instance name, e.g. `My Device._http._tcp.local`.
		instance: String,
	},
	/// `SRV <instance> -> <host>:<port>`: where to reach the instance.
	Srv {
		/// The instance name this record describes.
		name: String,
		/// The target host name, e.g. `my-device.local`.
		host: String,
		/// The TCP/UDP port the service listens on.
		port: u16,
		/// The record's TTL in seconds; `0` is a goodbye (the instance is
		/// leaving).
		ttl: u32,
	},
	/// `A <host> -> <ipv4>`: the host's address.
	A {
		/// The host name this address belongs to.
		name: String,
		/// The IPv4 address.
		addr: Ipv4Addr,
		/// The record's TTL in seconds; `0` is a goodbye.
		ttl: u32,
	},
	/// `TXT <instance> -> [key=value, ...]`: service metadata.
	Txt {
		/// The instance name this metadata belongs to.
		name: String,
		/// The raw `key=value` strings (mDNS TXT entries are length-prefixed
		/// byte strings; left undecoded beyond UTF-8 lossy).
		entries: Vec<String>,
	},
	/// Any other record type, kept only so the answer scan stays aligned.
	Other {
		/// The record name.
		name: String,
		/// The DNS record type.
		rtype: u16,
		/// The record's TTL in seconds.
		ttl: u32,
	},
}

/// The parsed records from an mDNS response (the answer + additional sections).
///
/// mDNS responders pack the `PTR`/`SRV`/`TXT`/`A` for a service across the
/// answer and additional sections, so [`parse_response`] flattens both into one
/// list and the browser correlates them by name.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MdnsResponse {
	/// All decoded records, answer + additional sections in order.
	pub records: Vec<Record>,
}

impl MdnsResponse {
	/// Iterate the `PTR` instances for `service_type`.
	pub fn ptr_instances<'a>(
		&'a self,
		service_type: &'a str,
	) -> impl Iterator<Item = &'a str> {
		self.records.iter().filter_map(move |rec| match rec {
			Record::Ptr { name, instance }
				if name.eq_ignore_ascii_case(service_type) =>
			{
				Some(instance.as_str())
			}
			_ => None,
		})
	}

	/// Find the `SRV` record for `instance`, if present.
	pub fn srv_for<'a>(&'a self, instance: &str) -> Option<&'a Record> {
		self.records.iter().find(|rec| {
			matches!(rec, Record::Srv { name, .. } if name.eq_ignore_ascii_case(instance))
		})
	}

	/// Find the first `A` record for `host`, if present.
	pub fn a_for<'a>(&'a self, host: &str) -> Option<&'a Record> {
		self.records.iter().find(|rec| {
			matches!(rec, Record::A { name, .. } if name.eq_ignore_ascii_case(host))
		})
	}

	/// Find the `TXT` record for `instance`, if present.
	pub fn txt_for<'a>(&'a self, instance: &str) -> Option<&'a Record> {
		self.records.iter().find(|rec| {
			matches!(rec, Record::Txt { name, .. } if name.eq_ignore_ascii_case(instance))
		})
	}
}

/// Build a multicast `PTR` query for `service_type` (e.g. `_http._tcp.local`)
/// into `buf`, returning its length, or `None` if it does not fit.
///
/// This is the question a browser multicasts to enumerate a service type;
/// responders reply with the `PTR`/`SRV`/`TXT`/`A` records [`parse_response`]
/// decodes.
pub fn build_ptr_query(buf: &mut [u8], service_type: &str) -> Option<usize> {
	let mut w = Writer::new(buf);
	w.u16(0)?; // id (0 for mDNS)
	w.u16(0)?; // flags: standard query
	w.u16(1)?; // qdcount
	w.u16(0)?; // ancount
	w.u16(0)?; // nscount
	w.u16(0)?; // arcount
	w.name(service_type)?;
	w.u16(TYPE_PTR)?;
	w.u16(CLASS_IN)?;
	Some(w.pos)
}

/// Parse an mDNS response packet into its [`Record`]s.
///
/// Returns `None` if the header is malformed; an empty/non-response packet
/// yields an empty record list. Tolerates compression pointers in names and
/// skips record types it does not decode.
pub fn parse_response(pkt: &[u8]) -> Option<MdnsResponse> {
	let mut r = Reader::new(pkt)?;
	// A response has QR=1; a pure query has no answers anyway, so we don't hard
	// require the QR bit (some stacks set odd flags) — we just read whatever
	// answer/additional records are present.
	let qd = r.qdcount;
	let an = r.ancount;
	let ns = r.nscount;
	let ar = r.arcount;

	// Skip the question section.
	for _ in 0..qd {
		r.skip_name()?;
		r.u16()?; // qtype
		r.u16()?; // qclass
	}

	let mut records = Vec::new();
	// Answer + authority + additional sections all carry RRs of the same shape.
	for _ in 0..(an as usize + ns as usize + ar as usize) {
		match r.read_record() {
			Some(record) => records.push(record),
			// A record we couldn't bounds-check cleanly: stop, keep what we have.
			None => break,
		}
	}

	Some(MdnsResponse { records })
}

/// Tiny forward-only writer over a fixed buffer; every method returns `None` on
/// overflow.
struct Writer<'a> {
	buf: &'a mut [u8],
	pos: usize,
}

impl<'a> Writer<'a> {
	fn new(buf: &'a mut [u8]) -> Self {
		Self { buf, pos: 0 }
	}
	fn u8(&mut self, v: u8) -> Option<()> {
		*self.buf.get_mut(self.pos)? = v;
		self.pos += 1;
		Some(())
	}
	fn u16(&mut self, v: u16) -> Option<()> {
		self.u8((v >> 8) as u8)?;
		self.u8(v as u8)
	}
	/// Write a dotted name (`_http._tcp.local`) as DNS labels terminated by a
	/// zero byte. Each label must be 1..=63 bytes.
	fn name(&mut self, name: &str) -> Option<()> {
		for part in name.split('.') {
			let bytes = part.as_bytes();
			if bytes.is_empty() || bytes.len() > 63 {
				return None;
			}
			self.u8(bytes.len() as u8)?;
			for &b in bytes {
				self.u8(b)?;
			}
		}
		self.u8(0)
	}
}

/// Forward-only reader over a received packet, holding the parsed header.
/// Bounds-checked: any out-of-range read returns `None`.
struct Reader<'a> {
	buf: &'a [u8],
	pos: usize,
	qdcount: u16,
	ancount: u16,
	nscount: u16,
	arcount: u16,
}

impl<'a> Reader<'a> {
	/// Parse the 12-byte header, leaving `pos` at the first question.
	fn new(buf: &'a [u8]) -> Option<Self> {
		if buf.len() < 12 {
			return None;
		}
		Some(Self {
			buf,
			pos: 12,
			qdcount: u16::from_be_bytes([buf[4], buf[5]]),
			ancount: u16::from_be_bytes([buf[6], buf[7]]),
			nscount: u16::from_be_bytes([buf[8], buf[9]]),
			arcount: u16::from_be_bytes([buf[10], buf[11]]),
		})
	}
	fn u8(&mut self) -> Option<u8> {
		let v = *self.buf.get(self.pos)?;
		self.pos += 1;
		Some(v)
	}
	fn u16(&mut self) -> Option<u16> {
		Some(u16::from_be_bytes([self.u8()?, self.u8()?]))
	}
	fn u32(&mut self) -> Option<u32> {
		Some(u32::from_be_bytes([
			self.u8()?,
			self.u8()?,
			self.u8()?,
			self.u8()?,
		]))
	}
	fn skip(&mut self, n: usize) -> Option<()> {
		self.pos = self.pos.checked_add(n)?;
		if self.pos > self.buf.len() {
			return None;
		}
		Some(())
	}

	/// Decode the DNS name at `pos` into a dotted `String`, following
	/// compression pointers. Advances `pos` past the name in the *current*
	/// stream (pointer jumps do not move the outer cursor past the pointer).
	fn read_name(&mut self) -> Option<String> {
		let mut out = String::new();
		// Where the outer cursor ends up: set once we follow the first pointer.
		let mut end: Option<usize> = None;
		let mut pos = self.pos;
		// Guard against pointer loops: a name can't be longer than the packet.
		let mut budget = self.buf.len();

		loop {
			if budget == 0 {
				return None;
			}
			budget -= 1;
			let len = *self.buf.get(pos)?;
			match len & 0xc0 {
				0xc0 => {
					// Compression pointer: 14-bit offset into the packet.
					let b2 = *self.buf.get(pos + 1)?;
					let offset = (((len & 0x3f) as usize) << 8) | b2 as usize;
					if end.is_none() {
						end = Some(pos + 2);
					}
					if offset >= self.buf.len() {
						return None;
					}
					pos = offset;
				}
				0x00 => {
					if len == 0 {
						pos += 1;
						break;
					}
					let n = len as usize;
					let start = pos + 1;
					let label = self.buf.get(start..start + n)?;
					if !out.is_empty() {
						out.push('.');
					}
					out.push_str(&String::from_utf8_lossy(label));
					pos = start + n;
				}
				// 0x40 / 0x80 are reserved — malformed.
				_ => return None,
			}
		}

		self.pos = end.unwrap_or(pos);
		Some(out)
	}

	/// Advance past a DNS name without decoding it (used for the question
	/// section). Tolerant of compression pointers.
	fn skip_name(&mut self) -> Option<()> {
		loop {
			let len = self.u8()?;
			match len & 0xc0 {
				0xc0 => {
					self.u8()?;
					return Some(());
				}
				0x00 => {
					if len == 0 {
						return Some(());
					}
					self.skip(len as usize)?;
				}
				_ => return None,
			}
		}
	}

	/// Read one resource record (name, type, class, ttl, rdata) and decode it
	/// into a [`Record`].
	fn read_record(&mut self) -> Option<Record> {
		let name = self.read_name()?;
		let rtype = self.u16()?;
		let _class = self.u16()?;
		let ttl = self.u32()?;
		let rdlen = self.u16()? as usize;
		let rdata_start = self.pos;
		let rdata_end = rdata_start.checked_add(rdlen)?;
		if rdata_end > self.buf.len() {
			return None;
		}

		let record = match rtype {
			TYPE_PTR => {
				let instance = self.read_name()?;
				Record::Ptr { name, instance }
			}
			TYPE_SRV => {
				let _priority = self.u16()?;
				let _weight = self.u16()?;
				let port = self.u16()?;
				let host = self.read_name()?;
				Record::Srv {
					name,
					host,
					port,
					ttl,
				}
			}
			TYPE_A => {
				if rdlen != 4 {
					Record::Other { name, rtype, ttl }
				} else {
					let a = self.u8()?;
					let b = self.u8()?;
					let c = self.u8()?;
					let d = self.u8()?;
					Record::A {
						name,
						addr: Ipv4Addr::new(a, b, c, d),
						ttl,
					}
				}
			}
			TYPE_TXT => {
				let mut entries = Vec::new();
				while self.pos < rdata_end {
					let len = self.u8()? as usize;
					if len == 0 {
						continue;
					}
					let start = self.pos;
					let bytes = self.buf.get(start..start + len)?;
					entries
						.push(String::from_utf8_lossy(bytes).into_owned());
					self.pos = start + len;
				}
				Record::Txt { name, entries }
			}
			_ => Record::Other { name, rtype, ttl },
		};

		// Always resume at the declared end of rdata, regardless of how much of
		// it we decoded (SRV/PTR rdata may use compression that the parser above
		// resolved past `rdata_end`).
		self.pos = rdata_end;
		Some(record)
	}
}

#[cfg(test)]
mod test {
	use super::*;

	/// Build an mDNS response carrying a PTR + SRV + A (+ optional TXT) for a
	/// single `_http._tcp.local` instance, the shape a real responder emits.
	/// Names are written uncompressed for test legibility; the parser tolerates
	/// both.
	fn build_browse_response(
		service_type: &str,
		instance: &str,
		host: &str,
		port: u16,
		addr: Ipv4Addr,
		ttl: u32,
		txt: &[&str],
	) -> Vec<u8> {
		let mut buf = Vec::new();
		// header: QR=1, AA=1
		buf.extend_from_slice(&0u16.to_be_bytes()); // id
		buf.extend_from_slice(&0x8400u16.to_be_bytes()); // flags
		buf.extend_from_slice(&0u16.to_be_bytes()); // qdcount
		let ancount: u16 = if txt.is_empty() { 3 } else { 4 };
		buf.extend_from_slice(&ancount.to_be_bytes()); // ancount
		buf.extend_from_slice(&0u16.to_be_bytes()); // nscount
		buf.extend_from_slice(&0u16.to_be_bytes()); // arcount

		fn write_name(buf: &mut Vec<u8>, name: &str) {
			for part in name.split('.') {
				buf.push(part.len() as u8);
				buf.extend_from_slice(part.as_bytes());
			}
			buf.push(0);
		}

		// PTR: service_type -> instance
		write_name(&mut buf, service_type);
		buf.extend_from_slice(&TYPE_PTR.to_be_bytes());
		buf.extend_from_slice(&CLASS_IN.to_be_bytes());
		buf.extend_from_slice(&ttl.to_be_bytes());
		let mut ptr_rdata = Vec::new();
		write_name(&mut ptr_rdata, instance);
		buf.extend_from_slice(&(ptr_rdata.len() as u16).to_be_bytes());
		buf.extend_from_slice(&ptr_rdata);

		// SRV: instance -> host:port
		write_name(&mut buf, instance);
		buf.extend_from_slice(&TYPE_SRV.to_be_bytes());
		buf.extend_from_slice(&CLASS_IN.to_be_bytes());
		buf.extend_from_slice(&ttl.to_be_bytes());
		let mut srv_rdata = Vec::new();
		srv_rdata.extend_from_slice(&0u16.to_be_bytes()); // priority
		srv_rdata.extend_from_slice(&0u16.to_be_bytes()); // weight
		srv_rdata.extend_from_slice(&port.to_be_bytes());
		write_name(&mut srv_rdata, host);
		buf.extend_from_slice(&(srv_rdata.len() as u16).to_be_bytes());
		buf.extend_from_slice(&srv_rdata);

		// A: host -> addr
		write_name(&mut buf, host);
		buf.extend_from_slice(&TYPE_A.to_be_bytes());
		buf.extend_from_slice(&CLASS_IN.to_be_bytes());
		buf.extend_from_slice(&ttl.to_be_bytes());
		buf.extend_from_slice(&4u16.to_be_bytes());
		buf.extend_from_slice(&addr.octets());

		// TXT (optional): instance -> entries
		if !txt.is_empty() {
			write_name(&mut buf, instance);
			buf.extend_from_slice(&TYPE_TXT.to_be_bytes());
			buf.extend_from_slice(&CLASS_IN.to_be_bytes());
			buf.extend_from_slice(&ttl.to_be_bytes());
			let mut txt_rdata = Vec::new();
			for entry in txt {
				txt_rdata.push(entry.len() as u8);
				txt_rdata.extend_from_slice(entry.as_bytes());
			}
			buf.extend_from_slice(&(txt_rdata.len() as u16).to_be_bytes());
			buf.extend_from_slice(&txt_rdata);
		}

		buf
	}

	#[beet_core::test]
	fn ptr_query_roundtrips_through_parse() {
		let mut buf = [0u8; 256];
		let len = build_ptr_query(&mut buf, "_http._tcp.local").unwrap();
		// header (12) + name + qtype(2) + qclass(2)
		(len > 12).xpect_true();
		// a query has no answers, so parse yields no records
		let parsed = parse_response(&buf[..len]).unwrap();
		parsed.records.is_empty().xpect_true();
	}

	#[beet_core::test]
	fn parses_ptr_srv_a() {
		let pkt = build_browse_response(
			"_http._tcp.local",
			"My Device._http._tcp.local",
			"my-device.local",
			8337,
			Ipv4Addr::new(192, 168, 1, 42),
			120,
			&[],
		);
		let resp = parse_response(&pkt).unwrap();

		// PTR -> instance
		let instances: Vec<_> =
			resp.ptr_instances("_http._tcp.local").collect();
		instances.xpect_eq(vec!["My Device._http._tcp.local"]);

		// SRV -> host + port
		match resp.srv_for("My Device._http._tcp.local").unwrap() {
			Record::Srv { host, port, .. } => {
				host.as_str().xpect_eq("my-device.local");
				(*port).xpect_eq(8337);
			}
			other => panic!("expected SRV, got {other:?}"),
		}

		// A -> ipv4
		match resp.a_for("my-device.local").unwrap() {
			Record::A { addr, .. } => {
				(*addr).xpect_eq(Ipv4Addr::new(192, 168, 1, 42));
			}
			other => panic!("expected A, got {other:?}"),
		}
	}

	#[beet_core::test]
	fn parses_txt_entries() {
		let pkt = build_browse_response(
			"_http._tcp.local",
			"My Device._http._tcp.local",
			"my-device.local",
			80,
			Ipv4Addr::new(10, 0, 0, 5),
			120,
			&["path=/", "v=1"],
		);
		let resp = parse_response(&pkt).unwrap();
		match resp.txt_for("My Device._http._tcp.local").unwrap() {
			Record::Txt { entries, .. } => {
				entries.clone().xpect_eq(vec![
					"path=/".to_string(),
					"v=1".to_string(),
				]);
			}
			other => panic!("expected TXT, got {other:?}"),
		}
	}

	#[beet_core::test]
	fn goodbye_has_zero_ttl() {
		let pkt = build_browse_response(
			"_http._tcp.local",
			"My Device._http._tcp.local",
			"my-device.local",
			8337,
			Ipv4Addr::new(192, 168, 1, 42),
			0, // goodbye
			&[],
		);
		let resp = parse_response(&pkt).unwrap();
		match resp.srv_for("My Device._http._tcp.local").unwrap() {
			Record::Srv { ttl, .. } => {
				(*ttl).xpect_eq(0);
			}
			other => panic!("expected SRV, got {other:?}"),
		}
	}

	#[beet_core::test]
	fn follows_compression_pointers() {
		// Hand-build a packet where the SRV's host name is a compression
		// pointer back to an A record's name, the way real responders compress.
		let mut buf = Vec::new();
		buf.extend_from_slice(&0u16.to_be_bytes()); // id
		buf.extend_from_slice(&0x8400u16.to_be_bytes()); // flags
		buf.extend_from_slice(&0u16.to_be_bytes()); // qdcount
		buf.extend_from_slice(&2u16.to_be_bytes()); // ancount (A, SRV)
		buf.extend_from_slice(&0u16.to_be_bytes()); // nscount
		buf.extend_from_slice(&0u16.to_be_bytes()); // arcount

		// A record: "host.local" at a known offset.
		let host_off = buf.len();
		for part in ["host", "local"] {
			buf.push(part.len() as u8);
			buf.extend_from_slice(part.as_bytes());
		}
		buf.push(0);
		buf.extend_from_slice(&TYPE_A.to_be_bytes());
		buf.extend_from_slice(&CLASS_IN.to_be_bytes());
		buf.extend_from_slice(&120u32.to_be_bytes());
		buf.extend_from_slice(&4u16.to_be_bytes());
		buf.extend_from_slice(&Ipv4Addr::new(1, 2, 3, 4).octets());

		// SRV record named "inst.local", whose target is a pointer to host_off.
		for part in ["inst", "local"] {
			buf.push(part.len() as u8);
			buf.extend_from_slice(part.as_bytes());
		}
		buf.push(0);
		buf.extend_from_slice(&TYPE_SRV.to_be_bytes());
		buf.extend_from_slice(&CLASS_IN.to_be_bytes());
		buf.extend_from_slice(&120u32.to_be_bytes());
		let mut srv_rdata = Vec::new();
		srv_rdata.extend_from_slice(&0u16.to_be_bytes()); // priority
		srv_rdata.extend_from_slice(&0u16.to_be_bytes()); // weight
		srv_rdata.extend_from_slice(&9999u16.to_be_bytes()); // port
		// compression pointer to host_off
		let ptr = 0xc000u16 | host_off as u16;
		srv_rdata.extend_from_slice(&ptr.to_be_bytes());
		buf.extend_from_slice(&(srv_rdata.len() as u16).to_be_bytes());
		buf.extend_from_slice(&srv_rdata);

		let resp = parse_response(&buf).unwrap();
		match resp.srv_for("inst.local").unwrap() {
			Record::Srv { host, port, .. } => {
				host.as_str().xpect_eq("host.local");
				(*port).xpect_eq(9999);
			}
			other => panic!("expected SRV, got {other:?}"),
		}
		match resp.a_for("host.local").unwrap() {
			Record::A { addr, .. } => {
				(*addr).xpect_eq(Ipv4Addr::new(1, 2, 3, 4));
			}
			other => panic!("expected A, got {other:?}"),
		}
	}

	#[beet_core::test]
	fn rejects_short_packet() {
		parse_response(&[0u8; 4]).is_none().xpect_true();
	}
}
