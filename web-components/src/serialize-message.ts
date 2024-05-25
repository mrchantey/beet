interface Message {
	SendEvent?: {
		reg_id: number
		payload: {
			Json: string
		}
	}
}

const messageLookup = {
	"beet_net::replication::common_events::AppReady": 0,
	"beet_examples::dialog_panel::OnPlayerMessage": 1
	// "web_event::MyEvent": 0,
	// "print_registry::MyEvent": 1,
	// "print_registry::MyResource": 2
	// "print_registry::MyComponent": 0,
	// "print_registry::MyEvent": 1,
	// "print_registry::MyResource": 2
}

export function sendEventMessage(messageKey: keyof typeof messageLookup, payload: any) {
	const reg_id = messageLookup[messageKey]
	send({
		SendEvent: {
			reg_id,
			payload: {
				Json: JSON.stringify(payload)
			}
		}
	})
}


function send(message: Message) {
	let detail = JSON.stringify(message)
	window.dispatchEvent(new CustomEvent('js-message', { detail }))
}

export function addEventMessageListener(messageKey: keyof typeof messageLookup, callback: (payload: any) => void) {
	window.addEventListener('wasm-message', (event) => {
		const message = event.detail
		if (message.SendEvent?.reg_id === messageLookup[messageKey]) {
			const payload = JSON.parse(message.SendEvent.payload.Json)
			callback(payload)
		}
	})
}

addEventMessageListener('beet_net::replication::common_events::AppReady', (payload) => {
	console.log('AppReady', payload)
})

declare global {
	interface Window {
		addEventListener(event: 'wasm-message', listener: (event: CustomEvent<Message>) => void): void
	}
}