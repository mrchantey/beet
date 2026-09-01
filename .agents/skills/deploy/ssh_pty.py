#!/usr/bin/env python3
"""Non-interactive pty driver for the beet ssh TUI (deploy skill check `d`).

Usage: ssh_pty.py <HOST> <PORT> '<SCRIPT>'

The script is a `;`-separated list of ops applied in order:

  w:<secs>      wait, draining output into the emulator (accepts floats)
  m:<col>,<row> SGR mouse press+release at a 1-indexed cell
  t:<text>      click the first cell of `text` in the CURRENT frame, exiting with
                the frame if it is absent. Prefer this over `m:` for anything
                whose position depends on page content: a sidebar entry moves
                every time a doc or a blog post is added, and a stale `m:` clicks
                whatever slid into that cell instead of failing.
  k:<keys>      send raw keys (`tab`, `enter`, `esc`, `down`, `up`, or literal text)

Output from ssh is fed through a small VT emulator so the FINAL 80x24 frame is
reconstructed (the TUI repaints in place, so a raw byte dump is unreadable) and
printed to stdout. Wide glyphs (the TUI renders titles fullwidth) occupy two
cells, matching the server's own layout maths.

Env:
  PTY_RULER=1   print 1-indexed col/row guides around the frame, for
                rediscovering cell coordinates when the layout shifts
  PTY_USER      ssh user (default `root`; the server accepts any)
  PTY_TERM      terminal name to advertise (default `xterm-256color`). Set
                `xterm-kitty` to exercise the kitty graphics path: the TUI only
                transmits rasters to a terminal that advertises support, so with
                the default a totally broken image path still renders a clean
                frame and check `d` passes.
  PTY_COLS      pty width  (default 80)
  PTY_ROWS      pty height (default 24). A raster bounds itself to its scroll
                port and a partly clipped one places its visible portion, so the
                24-row default draws images and is what the image check uses.
  PTY_RAW       write the undecoded pty stream to this path. The VT emulator
                discards APC (`ESC _ ... ESC \\`) sequences, which is exactly
                where the kitty image payload lives, so asserting on images
                means grepping the raw stream, not the frame.
"""

import fcntl
import os
import pty
import re
import select
import signal
import struct
import subprocess
import sys
import termios
import time
import unicodedata

COLS = int(os.environ.get("PTY_COLS", 80))
ROWS = int(os.environ.get("PTY_ROWS", 24))
TERM = os.environ.get("PTY_TERM", "xterm-256color")


class Screen:
	"""Minimal VT100/xterm emulator: enough of CSI to reconstruct a frame."""

	def __init__(self, cols=COLS, rows=ROWS):
		self.cols = cols
		self.rows = rows
		self.grid = [[" "] * cols for _ in range(rows)]
		self.col = 0
		self.row = 0
		self.saved = (0, 0)
		self.pending = b""

	def feed(self, data: bytes):
		"""Consume bytes, buffering any trailing partial escape/utf8 sequence."""
		data = self.pending + data
		self.pending = b""
		i = 0
		while i < len(data):
			byte = data[i]
			if byte == 0x1B:
				consumed = self._escape(data, i)
				if consumed is None:  # incomplete, wait for more
					self.pending = data[i:]
					return
				i = consumed
			elif byte in (0x07,):
				i += 1
			elif byte == 0x0D:
				self.col = 0
				i += 1
			elif byte == 0x0A:
				self._newline()
				i += 1
			elif byte == 0x08:
				self.col = max(0, self.col - 1)
				i += 1
			elif byte == 0x09:
				self.col = min(self.cols - 1, (self.col // 8 + 1) * 8)
				i += 1
			elif byte < 0x20:
				i += 1
			else:
				end = i
				while end < len(data) and data[end] >= 0x20 and data[end] != 0x1B:
					end += 1
				chunk = data[i:end]
				try:
					text = chunk.decode("utf-8")
				except UnicodeDecodeError:
					# split mid-codepoint: decode what we can, buffer the rest
					for cut in range(1, 4):
						try:
							text = chunk[:-cut].decode("utf-8")
							self.pending = chunk[-cut:]
							break
						except UnicodeDecodeError:
							continue
					else:
						text = chunk.decode("utf-8", "replace")
				self._write(text)
				i = end

	def _write(self, text: str):
		for char in text:
			width = 2 if unicodedata.east_asian_width(char) in ("W", "F") else 1
			if self.col + width > self.cols:
				self._newline()
				self.col = 0
			if 0 <= self.row < self.rows:
				self.grid[self.row][self.col] = char
				for pad in range(1, width):
					if self.col + pad < self.cols:
						self.grid[self.row][self.col + pad] = ""
			self.col += width

	def _newline(self):
		self.row += 1
		if self.row >= self.rows:
			self.row = self.rows - 1
			self.grid.pop(0)
			self.grid.append([" "] * self.cols)

	def _escape(self, data: bytes, i: int):
		"""Apply the escape sequence at `i`, returning the next index."""
		if i + 1 >= len(data):
			return None
		kind = data[i + 1]
		if kind == 0x5B:  # CSI
			match = re.compile(rb"\x1b\[([\x30-\x3f]*)([\x20-\x2f]*)([\x40-\x7e])").match(data, i)
			if not match:
				return None if len(data) - i < 32 else i + 2
			self._csi(match.group(1).decode(), match.group(3))
			return match.end()
		if kind in (0x5D, 0x50, 0x5E, 0x5F):  # OSC/DCS/PM/APC, ends at BEL or ST
			end = data.find(b"\x07", i)
			stop = data.find(b"\x1b\\", i)
			if end == -1 or (stop != -1 and stop < end):
				return None if stop == -1 else stop + 2
			return end + 1
		if kind in (0x28, 0x29, 0x2A, 0x2B, 0x23, 0x25):  # charset designators
			return None if i + 3 > len(data) else i + 3
		if kind == 0x37:  # DECSC
			self.saved = (self.row, self.col)
			return i + 2
		if kind == 0x38:  # DECRC
			self.row, self.col = self.saved
			return i + 2
		if kind == 0x4D:  # RI, reverse index
			self.row = max(0, self.row - 1)
			return i + 2
		return i + 2

	def _csi(self, params: str, final: bytes):
		private = params.startswith("?")
		nums = [int(part) if part.isdigit() else 0 for part in params.lstrip("?<>=").split(";")]
		first = nums[0] if nums else 0
		if private and final in (b"h", b"l"):
			return
		if final in (b"H", b"f"):
			self.row = max(0, (nums[0] if len(nums) > 0 and nums[0] else 1) - 1)
			self.col = max(0, (nums[1] if len(nums) > 1 and nums[1] else 1) - 1)
		elif final == b"A":
			self.row = max(0, self.row - max(1, first))
		elif final == b"B":
			self.row = min(self.rows - 1, self.row + max(1, first))
		elif final == b"C":
			self.col = min(self.cols - 1, self.col + max(1, first))
		elif final == b"D":
			self.col = max(0, self.col - max(1, first))
		elif final == b"G":
			self.col = max(0, max(1, first) - 1)
		elif final == b"d":
			self.row = max(0, max(1, first) - 1)
		elif final == b"J":
			self._erase_display(first)
		elif final == b"K":
			self._erase_line(first)
		elif final == b"L":
			for _ in range(max(1, first)):
				self.grid.insert(self.row, [" "] * self.cols)
				self.grid.pop()
		elif final == b"M":
			for _ in range(max(1, first)):
				self.grid.pop(self.row)
				self.grid.append([" "] * self.cols)
		elif final == b"P":
			row = self.grid[self.row]
			del row[self.col : self.col + max(1, first)]
			row.extend([" "] * (self.cols - len(row)))
		elif final == b"X":
			for offset in range(max(1, first)):
				if self.col + offset < self.cols:
					self.grid[self.row][self.col + offset] = " "
		self.row = min(max(self.row, 0), self.rows - 1)
		self.col = min(max(self.col, 0), self.cols - 1)

	def _erase_display(self, mode: int):
		if mode == 2 or mode == 3:
			self.grid = [[" "] * self.cols for _ in range(self.rows)]
		elif mode == 0:
			self._erase_line(0)
			for row in range(self.row + 1, self.rows):
				self.grid[row] = [" "] * self.cols
		elif mode == 1:
			self._erase_line(1)
			for row in range(0, self.row):
				self.grid[row] = [" "] * self.cols

	def _erase_line(self, mode: int):
		row = self.grid[self.row]
		if mode == 0:
			for col in range(self.col, self.cols):
				row[col] = " "
		elif mode == 1:
			for col in range(0, min(self.col + 1, self.cols)):
				row[col] = " "
		else:
			self.grid[self.row] = [" "] * self.cols

	def lines(self):
		return ["".join(cell for cell in row).rstrip() for row in self.grid]

	def find_cell(self, text: str):
		"""1-indexed (col, row) of the first cell of `text`, or None.

		Searched row-major against a cell-aligned view: the trailing cell of a
		wide glyph is stored as `""`, so a joined line's string index is not a
		column once a fullwidth title precedes the match on that row.
		"""
		for row_index, row in enumerate(self.grid):
			line = "".join(cell if cell else "\0" for cell in row)
			col_index = line.find(text)
			if col_index != -1:
				return col_index + 1, row_index + 1
		return None

	def render(self, ruler: bool):
		lines = self.lines()
		if not ruler:
			return "\n".join(lines)
		tens = "".join(str((col + 1) // 10 % 10) for col in range(self.cols))
		ones = "".join(str((col + 1) % 10) for col in range(self.cols))
		out = [f"     {tens}", f"     {ones}"]
		out.extend(f"r{row + 1:<3} {line}" for row, line in enumerate(lines))
		return "\n".join(out)


class Session:
	"""An ssh connection on an 80x24 pty, driven by script ops."""

	SSH_ARGS = [
		"-tt",
		"-o", "StrictHostKeyChecking=no",
		"-o", "UserKnownHostsFile=/dev/null",
		"-o", "ConnectTimeout=15",
		"-o", "LogLevel=ERROR",
	]

	KEYS = {
		"tab": b"\t",
		"stab": b"\x1b[Z",
		"enter": b"\r",
		"esc": b"\x1b",
		"up": b"\x1b[A",
		"down": b"\x1b[B",
		"right": b"\x1b[C",
		"left": b"\x1b[D",
		"space": b" ",
	}

	def __init__(self, host: str, port: str, user: str):
		self.screen = Screen()
		# the emulator drops APC, so keep the undecoded stream for image asserts
		raw_path = os.environ.get("PTY_RAW")
		self.raw = open(raw_path, "wb") if raw_path else None
		self.master, slave = pty.openpty()
		fcntl.ioctl(self.master, termios.TIOCSWINSZ, struct.pack("HHHH", ROWS, COLS, 0, 0))
		self.proc = subprocess.Popen(
			["ssh", *self.SSH_ARGS, "-p", str(port), f"{user}@{host}"],
			stdin=slave,
			stdout=slave,
			stderr=slave,
			start_new_session=True,
			env={**os.environ, "TERM": TERM},
		)
		os.close(slave)

	def drain(self, seconds: float):
		"""Pump the pty into the emulator for `seconds`."""
		deadline = time.monotonic() + seconds
		while True:
			remaining = deadline - time.monotonic()
			if remaining <= 0:
				return
			ready, _, _ = select.select([self.master], [], [], min(remaining, 0.2))
			if not ready:
				continue
			try:
				data = os.read(self.master, 65536)
			except OSError:
				return
			if not data:
				return
			self.screen.feed(data)
			if self.raw:
				self.raw.write(data)
				self.raw.flush()

	def send(self, data: bytes):
		os.write(self.master, data)

	def click(self, col: int, row: int):
		"""SGR mouse press+release at a 1-indexed cell."""
		self.send(f"\x1b[<0;{col};{row}M".encode())
		self.drain(0.15)
		self.send(f"\x1b[<0;{col};{row}m".encode())

	def run(self, script: str):
		for op in (part.strip() for part in script.split(";")):
			if not op:
				continue
			verb, _, arg = op.partition(":")
			if verb == "w":
				self.drain(float(arg))
			elif verb == "m":
				col, row = (int(part) for part in arg.split(","))
				self.click(col, row)
			elif verb == "t":
				cell = self.screen.find_cell(arg)
				if cell is None:
					raise SystemExit(
						f"no cell matching {arg!r} in the frame:\n"
						+ self.screen.render(True)
					)
				self.click(*cell)
			elif verb == "k":
				self.send(self.KEYS.get(arg, arg.encode()))
			else:
				raise SystemExit(f"unknown op: {op}")

	def close(self):
		try:
			self.send(b"\x03\x04")
			self.drain(0.3)
		except OSError:
			pass
		try:
			os.killpg(os.getpgid(self.proc.pid), signal.SIGTERM)
		except (ProcessLookupError, PermissionError):
			pass
		self.proc.wait(timeout=5)
		os.close(self.master)
		if self.raw:
			self.raw.close()


def main():
	if len(sys.argv) < 4:
		raise SystemExit(__doc__)
	host, port, script = sys.argv[1], sys.argv[2], sys.argv[3]
	session = Session(host, port, os.environ.get("PTY_USER", "root"))
	try:
		session.run(script)
	finally:
		frame = session.screen.render(os.environ.get("PTY_RULER") == "1")
		try:
			session.close()
		except Exception:
			pass
	print(frame)


if __name__ == "__main__":
	main()
