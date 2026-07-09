/**
Beet web analytics client.

- Analytics as in 'is my app working', not 'what toothpaste does this person use'.
- No personal data and no cross-session tracking: a single opaque, session-scoped
  id (cleared when the browser session ends) groups a visit's events.
- Reports page views (with how long each was viewed), clicks, max scroll depth,
  and client-side errors. Page views survive caching: the server never sees a
  cached load, but this script still beacons it.
- Posts to POST /analytics. A page view carries a page_view_id that overwrites the
  stored row, so the load, the periodic heartbeat, and the final report fold into
  one record whose duration is the total dwell. Other events are one-shot.
**/

// Heartbeat interval, so a lost `pagehide` (mobile, tab discard) still bounds the
// dwell to the last beat rather than losing it.
const HEARTBEAT_MS = 10000;

// A time-ordered UUIDv7 (48-bit unix-ms timestamp + random). Used for the
// page-view row id (so records sort by time and overwrite in place) and the
// session id. Built on `crypto.getRandomValues`, which - unlike
// `crypto.randomUUID` - is available in insecure contexts (plain http) too.
function uuidv7() {
	const ts = Date.now();
	const bytes = new Uint8Array(16);
	for (let i = 5; i >= 0; i--) {
		bytes[i] = Math.floor(ts / 2 ** (8 * (5 - i))) & 0xff;
	}
	crypto.getRandomValues(bytes.subarray(6));
	bytes[6] = (bytes[6] & 0x0f) | 0x70; // version 7
	bytes[8] = (bytes[8] & 0x3f) | 0x80; // variant
	const hex = [...bytes].map((b) => b.toString(16).padStart(2, "0"));
	return (
		hex.slice(0, 4).join("") +
		"-" +
		hex.slice(4, 6).join("") +
		"-" +
		hex.slice(6, 8).join("") +
		"-" +
		hex.slice(8, 10).join("") +
		"-" +
		hex.slice(10, 16).join("")
	);
}

// The opaque session id, generated once per browser session and mirrored into a
// session-scoped cookie so the server can attribute its own request log to the
// same session (a top-level navigation cannot carry a custom header).
function sessionId() {
	let id = sessionStorage.getItem("beet_session");
	if (!id) {
		id = uuidv7();
		sessionStorage.setItem("beet_session", id);
	}
	document.cookie = "beet_session=" + id + "; path=/; samesite=lax";
	return id;
}

// The client descriptor: what kind of client this is, no identifiers.
function clientDescriptor() {
	return {
		user_agent: navigator.userAgent,
		language: navigator.language,
		platform: navigator.platform,
		screen_width: screen.width,
		screen_height: screen.height,
		viewport_width: window.innerWidth,
		viewport_height: window.innerHeight,
		device_pixel_ratio: window.devicePixelRatio || 1,
		timezone: Intl.DateTimeFormat().resolvedOptions().timeZone,
	};
}

function createBeetAnalytics() {
	const session = sessionId();
	const client = clientDescriptor();
	const pageViewId = uuidv7();
	const startedAt = Date.now();
	let maxScroll = 0;

	// POST a payload to the analytics endpoint, beacon-first for reliability.
	// The Blob carries the json content type: `sendBeacon` with a plain string
	// would post `text/plain` and the body would not parse as a map.
	const post = (payload) => {
		const body = new Blob([JSON.stringify(payload)], {
			type: "application/json",
		});
		if ("sendBeacon" in navigator) {
			navigator.sendBeacon("/analytics", body);
		} else {
			fetch("/analytics", {
				method: "POST",
				body,
				keepalive: true,
			}).catch((err) => console.warn("Analytics send failed:", err));
		}
	};

	// A one-shot interaction/error event (click, scroll, error).
	const sendEvent = (kind, data) =>
		post(
			Object.assign(
				{ kind, session, path: window.location.pathname },
				data,
			),
		);

	// The page view: the same page_view_id overwrites the row, so the last post
	// (heartbeat or final) wins and duration_ms is the total dwell.
	const sendPageView = () =>
		post({
			kind: "page_view",
			page_view_id: pageViewId,
			session,
			path: window.location.pathname,
			duration_ms: Date.now() - startedAt,
			referrer: document.referrer || "direct",
			title: document.title,
			client,
		});

	// Track clicks on interactive elements (buttons and links).
	const trackClicks = () => {
		document.addEventListener("click", (event) => {
			const target = event.target;
			const tag = target.tagName;
			if (tag !== "BUTTON" && tag !== "A") return;
			sendEvent("click", {
				reason: tag === "A" ? "anchor-click" : "button-click",
				element: {
					tag,
					id: target.id,
					class: target.className,
					text: (target.textContent || "").slice(0, 128),
				},
			});
		});
	};

	// Track the max scroll depth reached, reported once at the end.
	const trackScroll = () => {
		window.addEventListener("scroll", () => {
			const scrollable = document.body.scrollHeight - window.innerHeight;
			if (scrollable <= 0) return;
			const percent = Math.round((window.scrollY / scrollable) * 100);
			if (percent > maxScroll) maxScroll = percent;
		});
	};

	// Track uncaught errors and intercepted console errors.
	const trackErrors = () => {
		window.addEventListener("error", (event) => {
			sendEvent("error", {
				source: "window",
				message: event.message,
				file: event.filename,
				line: event.lineno,
				column: event.colno,
			});
		});
		const original = console.error;
		console.error = function (...args) {
			sendEvent("error", {
				source: "console",
				message: args.map((arg) => String(arg)).join(" "),
			});
			original.apply(console, args);
		};
	};

	// Final report on leave: the page view's total dwell and the max scroll depth.
	const finalize = () => {
		clearInterval(heartbeat);
		sendPageView();
		if (maxScroll > 0) sendEvent("scroll", { max_percent: maxScroll });
	};

	// initial view (duration 0), a periodic heartbeat, and interaction tracking.
	sendPageView();
	const heartbeat = setInterval(sendPageView, HEARTBEAT_MS);
	trackClicks();
	trackScroll();
	trackErrors();
	// `pagehide` covers bfcache navigation; `visibilitychange -> hidden` is the
	// reliable mobile/tab-discard signal for an interim dwell update.
	window.addEventListener("pagehide", finalize);
	document.addEventListener("visibilitychange", () => {
		if (document.visibilityState === "hidden") sendPageView();
	});

	return { sendPageView, sendEvent, finalize };
}

createBeetAnalytics();
