/**
Basic Analytics Collection
This uses a flat structure with dot notation keys for maximum compatibility with various key-value store backends.
Some values may be arrays or objects but most are strings or numbers.
**/
class BeetAnalytics {
	constructor() {
		this.data = {};
		this.collectBasicInfo();
	}

	collectBasicInfo() {
		// Browser and OS information
		this.data["navigator.userAgent"] = navigator.userAgent;
		this.data["navigator.platform"] = navigator.platform;
		this.data["navigator.language"] = navigator.language;
		this.data["navigator.languages"] = navigator.languages || [
			navigator.language,
		];
		this.data["navigator.cookieEnabled"] = navigator.cookieEnabled;
		this.data["navigator.onLine"] = navigator.onLine;

		// Screen and display info
		this.data["screen.width"] = screen.width;
		this.data["screen.height"] = screen.height;
		this.data["screen.colorDepth"] = screen.colorDepth;
		this.data["screen.pixelDepth"] = screen.pixelDepth;
		this.data["window.innerWidth"] = window.innerWidth;
		this.data["window.innerHeight"] = window.innerHeight;
		this.data["window.devicePixelRatio"] = window.devicePixelRatio || 1;

		// Timezone and date info
		this.data["intl.timezone"] =
			Intl.DateTimeFormat().resolvedOptions().timeZone;
		this.data["date.timezoneOffset"] = new Date().getTimezoneOffset();
		this.data["session.created"] = Date.now();

		// Hardware info (when available)
		this.data["navigator.hardwareConcurrency"] =
			navigator.hardwareConcurrency || "unknown";
		this.data["navigator.maxTouchPoints"] = navigator.maxTouchPoints || 0;

		// Memory info (Chrome only)
		if ("memory" in performance) {
			this.data["performance.memory.limit"] =
				performance.memory.jsHeapSizeLimit;
			this.data["performance.memory.used"] = performance.memory.usedJSHeapSize;
		}

		// Connection info (when available)
		if ("connection" in navigator) {
			const conn = navigator.connection;
			this.data["connection.effectiveType"] = conn.effectiveType;
			this.data["connection.downlink"] = conn.downlink;
			this.data["connection.rtt"] = conn.rtt;
		}

		// Page and session info
		this.data["window.location.href"] = window.location.href;
		this.data["document.referrer"] = document.referrer || "direct";
		this.data["document.title"] = document.title;
		this.data["session.id"] = this.generateSessionId();

		// Performance timing
		if (performance.getEntriesByType) {
			const navTiming = performance.getEntriesByType("navigation")[0];
			if (navTiming) {
				this.data["performance.pageLoadTime"] =
					navTiming.loadEventEnd - navTiming.startTime;
				this.data["performance.domContentLoaded"] =
					navTiming.domContentLoadedEventEnd - navTiming.startTime;
			}
		}

		return this.data;
	}

	// Generate a session ID (not personally identifiable)
	generateSessionId() {
		return "sess_" + Date.now() + "_" + Math.random().toString(36).substr(2, 9);
	}

	// Track page interactions
	trackPageView() {
		this.sendEvent(
			"page_view",
			// these are duplicated but send anyway, may have changed
			{
				"event.window.location.href": window.location.href,
				"event.document.title": document.title,
			},
		);
	}

	trackClick(element, customData = {}) {
		this.sendEvent("click", {
			"event.element.tagName": element.tagName,
			"event.element.className": element.className,
			"event.element.id": element.id,
			"event.element.text": element.textContent?.substring(0, 128), // Limit text length
			...customData,
		});
	}

	trackScroll() {
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
			this.sendEvent("scroll_depth", { "event.scroll.maxPercent": maxScroll });
		});
	}

	// Simple error tracking
	trackErrors() {
		window.addEventListener("error", (event) => {
			this.sendEvent("javascript_error", {
				"event.error.message": event.message,
				"event.error.filename": event.filename,
				"event.error.line": event.lineno,
				"event.error.column": event.colno,
			});
		});
	}

	trackConsoleErrors() {
		const originalConsoleError = console.error;
		console.error = function (...args) {
			this.sendEvent("console_error", {
				"event.error.message": args.map((a) => String(a)).join(" "),
			});
			originalConsoleError.apply(console, args);
		};
	}

	trackClicks() {
		document.addEventListener("click", (e) => {
			if (e.target.tagName === "BUTTON" || e.target.tagName === "A") {
				self.trackClick(e.target);
			}
		});
	}

	// Send data to your analytics endpoint
	sendEvent(event_type, event_data = {}) {
		const payload = Object.assign(
			{
				"event.type": event_type,
				"event.client.timestamp": Date.now(),
			},
			this.data,
			event_data,
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
	}
}

// Initialize analytics
const analytics = new BeetAnalytics();
analytics.trackPageView();
analytics.trackClicks();
analytics.trackScroll();
analytics.trackErrors();
analytics.trackConsoleErrors();

// Example usage for custom events
// analytics.sendEvent('signup_attempt', { 'event.method': 'email' });
// analytics.sendEvent('purchase', { 'event.value': 29.99, 'event.currency': 'USD' });
