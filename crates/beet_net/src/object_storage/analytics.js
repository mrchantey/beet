// Basic Analytics Collection
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
		if ("performance" in window && "timing" in performance) {
			const timing = performance.timing;
			this.data["performance.pageLoadTime"] =
				timing.loadEventEnd - timing.navigationStart;
			this.data["performance.domContentLoaded"] =
				timing.domContentLoadedEventEnd - timing.navigationStart;
		}

		return this.data;
	}

	// Generate a session ID (not personally identifiable)
	generateSessionId() {
		return "sess_" + Date.now() + "_" + Math.random().toString(36).substr(2, 9);
	}

	// Track page interactions
	trackPageView() {
		this.sendEvent("page_view", {
			"window.location.href": window.location.href,
			"document.title": document.title,
		});
	}

	trackClick(element, customData = {}) {
		this.sendEvent("click", {
			"element.tagName": element.tagName,
			"element.className": element.className,
			"element.id": element.id,
			"element.text": element.textContent?.substring(0, 128), // Limit text length
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
			this.sendEvent("scroll_depth", { "scroll.maxPercent": maxScroll });
		});
	}

	// Simple error tracking
	trackErrors() {
		window.addEventListener("error", (event) => {
			this.sendEvent("javascript_error", {
				"error.message": event.message,
				"error.filename": event.filename,
				"error.line": event.lineno,
				"error.column": event.colno,
			});
		});
	}

	// Send data to your analytics endpoint
	sendEvent(event_type, event_data = {}) {
		const payload = {
			"event.type": event_type,
			"event.data": event_data,
			"session.data": this.data,
			"client.timestamp": Date.now(),
		};

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

// Track page view
analytics.trackPageView();

// Track clicks on important elements
document.addEventListener("click", (e) => {
	if (e.target.tagName === "BUTTON" || e.target.tagName === "A") {
		analytics.trackClick(e.target);
	}
});

// Track scroll depth
analytics.trackScroll();

// Track JavaScript errors
analytics.trackErrors();

// Example usage for custom events
// analytics.sendEvent('signup_attempt', { method: 'email' });
// analytics.sendEvent('purchase', { value: 29.99, currency: 'USD' });
