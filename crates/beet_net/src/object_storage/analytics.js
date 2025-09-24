// Basic Analytics Collection
class BeetAnalytics {
	constructor() {
		this.data = {};
		this.collectBasicInfo();
	}

	collectBasicInfo() {
		// Browser and OS information
		this.data.userAgent = navigator.userAgent;
		this.data.platform = navigator.platform;
		this.data.language = navigator.language;
		this.data.languages = navigator.languages || [navigator.language];
		this.data.cookieEnabled = navigator.cookieEnabled;
		this.data.onLine = navigator.onLine;

		// Screen and display info
		this.data.screenWidth = screen.width;
		this.data.screenHeight = screen.height;
		this.data.screenColorDepth = screen.colorDepth;
		this.data.screenPixelDepth = screen.pixelDepth;
		this.data.viewportWidth = window.innerWidth;
		this.data.viewportHeight = window.innerHeight;
		this.data.devicePixelRatio = window.devicePixelRatio || 1;

		// Timezone and date info
		this.data.timezone = Intl.DateTimeFormat().resolvedOptions().timeZone;
		this.data.timezoneOffset = new Date().getTimezoneOffset();
		this.data.timestamp = new Date().toISOString();

		// Hardware info (when available)
		this.data.hardwareConcurrency = navigator.hardwareConcurrency || "unknown";
		this.data.maxTouchPoints = navigator.maxTouchPoints || 0;

		// Memory info (Chrome only)
		if ("memory" in performance) {
			this.data.memoryLimit = performance.memory.jsHeapSizeLimit;
			this.data.memoryUsed = performance.memory.usedJSHeapSize;
		}

		// Connection info (when available)
		if ("connection" in navigator) {
			const conn = navigator.connection;
			this.data.connectionType = conn.effectiveType;
			this.data.downlink = conn.downlink;
			this.data.rtt = conn.rtt;
		}

		// Page and session info
		this.data.url = window.location.href;
		this.data.referrer = document.referrer || "direct";
		this.data.title = document.title;
		this.data.sessionId = this.generateSessionId();

		// Performance timing
		if ("performance" in window && "timing" in performance) {
			const timing = performance.timing;
			this.data.pageLoadTime = timing.loadEventEnd - timing.navigationStart;
			this.data.domContentLoaded =
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
			url: window.location.href,
			title: document.title,
			referrer: document.referrer || "direct",
			timestamp: new Date().toISOString(),
		});
	}

	trackClick(element, customData = {}) {
		this.sendEvent("click", {
			tagName: element.tagName,
			className: element.className,
			id: element.id,
			text: element.textContent?.substring(0, 128), // Limit text length
			timestamp: new Date().toISOString(),
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
			this.sendEvent("scroll_depth", { maxScrollPercent: maxScroll });
		});
	}

	// Simple error tracking
	trackErrors() {
		window.addEventListener("error", (event) => {
			this.sendEvent("javascript_error", {
				message: event.message,
				filename: event.filename,
				line: event.lineno,
				column: event.colno,
				timestamp: new Date().toISOString(),
			});
		});
	}

	// Send data to your analytics endpoint
	sendEvent(event_type, event_data = {}) {
		const payload = {
			event_type,
			event_data,
			session_data: this.data,
			client_timestamp: performance.now(),
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
