//! WebSocket ([RFC 6455]) frame codec and client handshake helpers.
//!
//! Pure byte-buffer work over beet's [`Message`], no transport: the socket
//! analogue of [`http_ext`](super::http_ext). A bare-metal client (esp) drives
//! its own TCP socket and uses these to frame [`Message`]s onto the wire and
//! decode them back, mirroring the `impl_tungstenite` message mapping.
//!
//! ## Limitations
//!
//! A first cut: it does not reassemble fragmented (continuation) frames and caps
//! decode at 16-bit payload lengths. Both are rejected cleanly as errors rather
//! than misparsed. The `beet socket-server` echo endpoint we test against never
//! fragments, so this covers the round-trip loop.
//!
//! [RFC 6455]: https://datatracker.ietf.org/doc/html/rfc6455
use crate::prelude::*;
use crate::sockets::CloseFrame;
use crate::sockets::Message;
use beet_core::prelude::*;

/// [RFC 6455 §5.2] frame opcodes.
///
/// [RFC 6455 §5.2]: https://datatracker.ietf.org/doc/html/rfc6455#section-5.2
mod opcode {
	pub const CONTINUATION: u8 = 0x0;
	pub const TEXT: u8 = 0x1;
	pub const BINARY: u8 = 0x2;
	pub const CLOSE: u8 = 0x8;
	pub const PING: u8 = 0x9;
	pub const PONG: u8 = 0xA;
}

/// Encode a [`Message`] as a single [RFC 6455 §5.2] frame, FIN set.
///
/// `mask` is `Some(key)` for a client (payloads MUST be masked, §5.3) and `None`
/// for a server. The key is supplied by the caller (a hardware RNG on esp) so
/// this stays pure and testable.
///
/// [RFC 6455 §5.2]: https://datatracker.ietf.org/doc/html/rfc6455#section-5.2
pub fn encode_frame(msg: &Message, mask: Option<[u8; 4]>) -> Vec<u8> {
	let (opcode, payload) = message_payload(msg);
	let mut frame = Vec::with_capacity(payload.len() + 14);
	// FIN + opcode
	frame.push(0x80 | opcode);
	// payload length in the 7-bit / 16-bit / 64-bit form, with the mask bit
	let mask_bit = if mask.is_some() { 0x80 } else { 0x00 };
	let len = payload.len();
	if len < 126 {
		frame.push(mask_bit | len as u8);
	} else if len <= u16::MAX as usize {
		frame.push(mask_bit | 126);
		frame.extend_from_slice(&(len as u16).to_be_bytes());
	} else {
		frame.push(mask_bit | 127);
		frame.extend_from_slice(&(len as u64).to_be_bytes());
	}
	// masked client payloads carry the 4-byte key then the XORed bytes (§5.3)
	match mask {
		Some(key) => {
			frame.extend_from_slice(&key);
			frame.extend(
				payload
					.iter()
					.enumerate()
					.map(|(i, byte)| byte ^ key[i % 4]),
			);
		}
		None => frame.extend_from_slice(&payload),
	}
	frame
}

/// Decode one [RFC 6455 §5.2] frame from the front of `buf`, returning the
/// [`Message`] and the number of bytes consumed, or `None` if `buf` holds only a
/// partial frame (read more, then retry).
///
/// Handles masked and unmasked frames and the 7/16/64-bit length forms. Rejects
/// fragmented (continuation) frames and reserved opcodes as errors rather than
/// misparsing; see the module limitations.
///
/// [RFC 6455 §5.2]: https://datatracker.ietf.org/doc/html/rfc6455#section-5.2
pub fn parse_frame(buf: &[u8]) -> Result<Option<(Message, usize)>> {
	if buf.len() < 2 {
		return Ok(None);
	}
	let fin = buf[0] & 0x80 != 0;
	if buf[0] & 0x70 != 0 {
		bevybail!(
			"WebSocket frame set a reserved bit (RSV1-3), no extension negotiated"
		);
	}
	let opcode = buf[0] & 0x0F;
	let masked = buf[1] & 0x80 != 0;

	// resolve the payload length and the offset it starts at (§5.2)
	let (payload_len, mut offset) = match buf[1] & 0x7F {
		126 => {
			if buf.len() < 4 {
				return Ok(None);
			}
			(u16::from_be_bytes([buf[2], buf[3]]) as usize, 4)
		}
		127 => {
			if buf.len() < 10 {
				return Ok(None);
			}
			let len = u64::from_be_bytes(buf[2..10].try_into().unwrap());
			if len > u16::MAX as u64 {
				bevybail!(
					"WebSocket frame payload {len} exceeds this decoder's 16-bit cap"
				);
			}
			(len as usize, 10)
		}
		short => (short as usize, 2),
	};

	// a masked frame prepends its 4-byte key before the payload (§5.3)
	let mask_key = masked
		.then(|| {
			(buf.len() >= offset + 4).then(|| {
				let key =
					[buf[offset], buf[offset + 1], buf[offset + 2], buf[offset + 3]];
				offset += 4;
				key
			})
		})
		.flatten();
	if masked && mask_key.is_none() {
		return Ok(None);
	}

	let end = offset + payload_len;
	if buf.len() < end {
		return Ok(None);
	}
	let mut payload = buf[offset..end].to_vec();
	if let Some(key) = mask_key {
		for (i, byte) in payload.iter_mut().enumerate() {
			*byte ^= key[i % 4];
		}
	}

	if !fin {
		bevybail!(
			"fragmented WebSocket frames are not supported (opcode {opcode:#x})"
		);
	}
	Ok(Some((decode_payload(opcode, payload)?, end)))
}

/// Encode 16 random bytes as a client `Sec-WebSocket-Key` ([RFC 6455 §4.1]).
///
/// [RFC 6455 §4.1]: https://datatracker.ietf.org/doc/html/rfc6455#section-4.1
pub fn encode_client_key(random: [u8; 16]) -> String {
	use base64::Engine as _;
	base64::engine::general_purpose::STANDARD.encode(random)
}

/// Build the raw HTTP/1.1 client handshake request bytes ([RFC 6455 §4.1]) for
/// `host`/`path` with the given `Sec-WebSocket-Key`.
///
/// Reuses [`http_ext::encode_request`] with `close_connection: false` (an upgrade
/// keeps the connection) and `content_length: false` (no body), so the wire
/// framing stays in one encoder.
///
/// [RFC 6455 §4.1]: https://datatracker.ietf.org/doc/html/rfc6455#section-4.1
pub fn encode_handshake_request(
	host: &str,
	path: &str,
	key: &str,
) -> Result<Vec<u8>> {
	let path = if path.is_empty() { "/" } else { path };
	let request = Request::get(format!("http://{host}{path}"))
		.with_header_raw("Upgrade", "websocket")
		.with_header_raw("Connection", "Upgrade")
		.with_header_raw("Sec-WebSocket-Key", key)
		.with_header_raw("Sec-WebSocket-Version", "13");
	http_ext::encode_request(
		&request,
		http_ext::EncodeRequestOptions {
			close_connection: false,
			content_length: false,
		},
	)
}

/// Validate the server's handshake response against the sent `Sec-WebSocket-Key`
/// ([RFC 6455 §4.1]): a `101` upgrade whose `Sec-WebSocket-Accept` equals
/// [`http_ext::sec_websocket_accept`] of the key.
///
/// [RFC 6455 §4.1]: https://datatracker.ietf.org/doc/html/rfc6455#section-4.1
pub fn validate_handshake_response(raw: &[u8], key: &str) -> Result<()> {
	let response = http_ext::parse_response(raw)?;
	if !http_ext::is_websocket_response(&response) {
		bevybail!(
			"server rejected the WebSocket upgrade: status {:?}",
			response.status()
		);
	}
	let expected = http_ext::sec_websocket_accept(key);
	match response.headers().first_raw("sec-websocket-accept") {
		Some(actual) if actual == expected => Ok(()),
		Some(actual) => bevybail!(
			"Sec-WebSocket-Accept mismatch: expected {expected}, got {actual}"
		),
		None => bevybail!("server handshake response missing Sec-WebSocket-Accept"),
	}
}

/// The opcode and payload bytes for a [`Message`] (§5.5, §5.6, §7.4).
fn message_payload(msg: &Message) -> (u8, Vec<u8>) {
	match msg {
		Message::Text(text) => (opcode::TEXT, text.as_bytes().to_vec()),
		Message::Binary(bytes) => (opcode::BINARY, bytes.to_vec()),
		Message::Ping(bytes) => (opcode::PING, bytes.to_vec()),
		Message::Pong(bytes) => (opcode::PONG, bytes.to_vec()),
		Message::Close(frame) => (opcode::CLOSE, close_payload(frame)),
	}
}

/// A close frame's payload: a 2-byte big-endian code + UTF-8 reason (§5.5.1), or
/// empty for a bare close.
fn close_payload(frame: &Option<CloseFrame>) -> Vec<u8> {
	match frame {
		Some(CloseFrame { code, reason }) => {
			let mut payload = Vec::with_capacity(2 + reason.len());
			payload.extend_from_slice(&code.to_be_bytes());
			payload.extend_from_slice(reason.as_bytes());
			payload
		}
		None => Vec::new(),
	}
}

/// Map a frame's opcode + unmasked payload to a [`Message`] (§5.5, §5.6).
fn decode_payload(opcode: u8, payload: Vec<u8>) -> Result<Message> {
	match opcode {
		opcode::TEXT => Ok(Message::Text(String::from_utf8(payload)?)),
		opcode::BINARY => Ok(Message::Binary(payload.into())),
		opcode::PING => Ok(Message::Ping(payload.into())),
		opcode::PONG => Ok(Message::Pong(payload.into())),
		opcode::CLOSE => Ok(Message::Close(parse_close(payload))),
		opcode::CONTINUATION => {
			bevybail!("received a continuation frame; fragmentation is not supported")
		}
		other => bevybail!("unknown WebSocket opcode {other:#x}"),
	}
}

/// Parse a close frame's payload into an optional [`CloseFrame`] (§5.5.1): a
/// 2-byte code + UTF-8 reason, or `None` when empty.
fn parse_close(payload: Vec<u8>) -> Option<CloseFrame> {
	if payload.len() < 2 {
		return None;
	}
	Some(CloseFrame {
		code: u16::from_be_bytes([payload[0], payload[1]]),
		reason: String::from_utf8_lossy(&payload[2..]).into_owned(),
	})
}

#[cfg(test)]
mod tests {
	use super::*;

	/// Encode a client (masked) frame, decode it, expect the original back.
	fn roundtrip(msg: Message) {
		let frame = encode_frame(&msg, Some([0x12, 0x34, 0x56, 0x78]));
		let (decoded, consumed) = parse_frame(&frame).unwrap().unwrap();
		consumed.xpect_eq(frame.len());
		decoded.xpect_eq(msg);
	}

	#[beet_core::test]
	fn frame_roundtrip_each_variant() {
		roundtrip(Message::text("the cat sat on the"));
		roundtrip(Message::binary(vec![0u8, 1, 2, 255, 128]));
		roundtrip(Message::ping(vec![9u8, 9]));
		roundtrip(Message::pong(vec![]));
		roundtrip(Message::Close(Some(CloseFrame {
			code: 1000,
			reason: "bye".into(),
		})));
		roundtrip(Message::Close(None));
	}

	#[beet_core::test]
	fn client_frame_is_masked() {
		let key = [0xAAu8, 0xBB, 0xCC, 0xDD];
		let frame = encode_frame(&Message::text("hi"), Some(key));
		// FIN + text opcode
		frame[0].xpect_eq(0x81u8);
		// mask bit + payload len 2
		frame[1].xpect_eq(0x82u8);
		frame[2..6].to_vec().xpect_eq(key.to_vec());
		frame[6..8].to_vec().xpect_eq(vec![b'h' ^ key[0], b'i' ^ key[1]]);
	}

	#[beet_core::test]
	fn parses_unmasked_server_frame() {
		let frame = encode_frame(&Message::text("hat"), None);
		// no mask bit, len 3
		frame[1].xpect_eq(3u8);
		let (msg, consumed) = parse_frame(&frame).unwrap().unwrap();
		consumed.xpect_eq(frame.len());
		msg.xpect_eq(Message::text("hat"));
	}

	#[beet_core::test]
	fn partial_frame_needs_more() {
		let frame = encode_frame(&Message::text("hello"), None);
		// one byte short: the decoder asks for more rather than misparsing.
		parse_frame(&frame[..frame.len() - 1])
			.unwrap()
			.is_none()
			.xpect_true();
		parse_frame(&[]).unwrap().is_none().xpect_true();
		parse_frame(&frame[..1]).unwrap().is_none().xpect_true();
	}

	#[beet_core::test]
	fn rejects_fragmented_and_continuation() {
		// opcode 0x0 (continuation), FIN set, empty payload
		parse_frame(&[0x80, 0x00]).unwrap_err();
		// a non-FIN data frame (would need reassembly)
		parse_frame(&[0x01, 0x00]).unwrap_err();
	}

	#[beet_core::test]
	fn computes_rfc_accept_key() {
		// RFC 6455 §1.3 worked example.
		http_ext::sec_websocket_accept("dGhlIHNhbXBsZSBub25jZQ==")
			.xpect_eq("s3pPLMBiTxaQ9kYGzzhZRbK+xOo=");
	}

	#[beet_core::test]
	fn builds_and_parses_handshake_request() {
		let key = encode_client_key([7u8; 16]);
		let raw = encode_handshake_request("127.0.0.1:8338", "/", &key).unwrap();
		let request = http_ext::parse_http_request(&raw).unwrap();
		request
			.headers()
			.first_raw("upgrade")
			.unwrap()
			.xpect_eq("websocket");
		request
			.headers()
			.first_raw("connection")
			.unwrap()
			.xpect_eq("Upgrade");
		request
			.headers()
			.first_raw("sec-websocket-key")
			.unwrap()
			.xpect_eq(key.as_str());
		request
			.headers()
			.first_raw("sec-websocket-version")
			.unwrap()
			.xpect_eq("13");
	}

	#[beet_core::test]
	fn validates_server_handshake() {
		let key = "dGhlIHNhbXBsZSBub25jZQ==";
		let accept = http_ext::sec_websocket_accept(key);
		let raw = format!(
			"HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\n\
			 Connection: Upgrade\r\nSec-WebSocket-Accept: {accept}\r\n\r\n"
		);
		validate_handshake_response(raw.as_bytes(), key).unwrap();
		// a wrong accept key is rejected.
		let bad = raw.replace(&accept, "wrongaccept");
		validate_handshake_response(bad.as_bytes(), key).unwrap_err();
	}
}
