/**
Basic Analytics Collection

- Analytics as in 'is my app working' not 'lets find out what toothpaste this person uses'
- Fully GDPR compliant - no personal data, no cookies, no tracking
- Uses a flat structure with dot notation keys for maximum compatibility with various key-value store backends.
- Some values may be arrays or objects but most are strings or numbers.
**/

// Collect all basic browser/device information
function createSessionData() {
	const sessionData = {};
	// Browser and OS information
	sessionData["navigator.userAgent"] = navigator.userAgent;
	sessionData["navigator.platform"] = navigator.platform;
	sessionData["navigator.language"] = navigator.language;
	sessionData["navigator.languages"] = navigator.languages || [
		navigator.language,
	];
	sessionData["navigator.cookieEnabled"] = navigator.cookieEnabled;
	sessionData["navigator.onLine"] = navigator.onLine;

	// Screen and display info
	sessionData["screen.width"] = screen.width;
	sessionData["screen.height"] = screen.height;
	sessionData["screen.colorDepth"] = screen.colorDepth;
	sessionData["screen.pixelDepth"] = screen.pixelDepth;
	sessionData["window.innerWidth"] = window.innerWidth;
	sessionData["window.innerHeight"] = window.innerHeight;
	sessionData["window.devicePixelRatio"] = window.devicePixelRatio || 1;

	// Timezone and date info
	sessionData["intl.timezone"] =
		Intl.DateTimeFormat().resolvedOptions().timeZone;
	sessionData["date.timezoneOffset"] = new Date().getTimezoneOffset();
	sessionData["session.created"] = Date.now();

	// Hardware info (when available)
	sessionData["navigator.hardwareConcurrency"] =
		navigator.hardwareConcurrency || "unknown";
	sessionData["navigator.maxTouchPoints"] = navigator.maxTouchPoints || 0;

	// Memory info (Chrome only)
	if ("memory" in performance) {
		sessionData["performance.memory.limit"] =
			performance.memory.jsHeapSizeLimit;
		sessionData["performance.memory.used"] = performance.memory.usedJSHeapSize;
	}

	// Connection info (when available)
	if ("connection" in navigator) {
		const conn = navigator.connection;
		sessionData["connection.effectiveType"] = conn.effectiveType;
		sessionData["connection.downlink"] = conn.downlink;
		sessionData["connection.rtt"] = conn.rtt;
	}

	// Page and session info
	sessionData["window.location.href"] = window.location.href;
	sessionData["document.referrer"] = document.referrer || "direct";
	sessionData["document.title"] = document.title;
	// Generate a session ID (not personally identifiable)
	sessionData["session.id"] =
		"sess_" + Date.now() + "_" + Math.random().toString(36).substr(2, 9);

	// Performance timing
	if (performance.getEntriesByType) {
		const navTiming = performance.getEntriesByType("navigation")[0];
		if (navTiming) {
			sessionData["performance.pageLoadTime"] =
				navTiming.loadEventEnd - navTiming.startTime;
			sessionData["performance.domContentLoaded"] =
				navTiming.domContentLoadedEventEnd - navTiming.startTime;
		}
	}

	return sessionData;
}

function createBeetAnalytics() {
	// Internal data store - completely private
	const sessionData = createSessionData();

	// Send data to analytics endpoint
	const sendEvent = (eventType, eventData = {}) => {
		// flat structure
		const payload = Object.assign(
			{
				"event.type": eventType,
				"event.client.timestamp": Date.now(),
			},
			sessionData,
			eventData,
		);

		// Use beacon API for better reliability
		if ("sendBeacon" in navigator) {
			navigator.sendBeacon("/analytics", JSON.stringify(payload));
		} else {
			// Fallback to fetch
			fetch("/analytics", {
				method: "POST",
				headers: { "Content-Type": "application/json" },
				body: JSON.stringify(payload),
				keepalive: true,
			}).catch((err) => console.warn("Analytics send failed:", err));
		}
	};

	// Track page interactions
	const sendPageView = () => {
		sendEvent(
			"page_view",
			// these are duplicated but send anyway, may have changed
			{
				"event.window.location.href": window.location.href,
				"event.document.title": document.title,
			},
		);
	};

	const sendClick = (element, customData = {}) => {
		sendEvent("click", {
			"event.element.tagName": element.tagName,
			"event.element.className": element.className,
			"event.element.id": element.id,
			"event.element.text": element.textContent?.substring(0, 128), // Limit text length
			...customData,
		});
	};

	const trackScroll = () => {
		let maxScroll = 0;
		window.addEventListener("scroll", () => {
			const scrollPercent = Math.round(
				(window.scrollY / (document.body.scrollHeight - window.innerHeight)) *
					100,
			);
			if (scrollPercent > maxScroll) {
				maxScroll = scrollPercent;
			}
		});

		// Send max scroll on page unload
		window.addEventListener("beforeunload", () => {
			sendEvent("scroll_depth", { "event.scroll.maxPercent": maxScroll });
		});
	};

	// Simple error tracking
	const trackErrors = () => {
		window.addEventListener("error", (event) => {
			sendEvent("error/event", {
				"event.error.message": event.message,
				"event.error.filename": event.filename,
				"event.error.line": event.lineno,
				"event.error.column": event.colno,
			});
		});
	};

	const trackConsoleErrors = () => {
		const originalConsoleError = console.error;
		console.error = function (...args) {
			sendEvent("error/log", {
				"event.error.message": args.map((a) => String(a)).join(" "),
			});
			originalConsoleError.apply(console, args);
		};
	};

	const trackClicks = () => {
		document.addEventListener("click", (e) => {
			if (e.target.tagName === "BUTTON") {
				sendClick(e.target, { "event.reason": "button-click" });
			} else if (e.target.tagName === "A") {
				sendClick(e.target, { "event.reason": "anchor-click" });
			}
		});
	};

	// Return public interface - no 'this' anywhere!
	return {
		sendEvent,
		sendPageView,
		sendClick,
		trackScroll,
		trackErrors,
		trackConsoleErrors,
		trackClicks,
	};
}

// Initialize analytics
const beetAnalytics = createBeetAnalytics();
beetAnalytics.sendPageView();
beetAnalytics.trackClicks();
beetAnalytics.trackScroll();
beetAnalytics.trackErrors();
beetAnalytics.trackConsoleErrors();

// Example usage for custom events
// beetAnalytics.sendEvent('signup_attempt', { 'event.method': 'email' });
// beetAnalytics.sendEvent('purchase', { 'event.value': 29.99, 'event.currency': 'USD' });
