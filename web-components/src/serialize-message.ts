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
	let detail = JSON.stringify([message])
	window.dispatchEvent(new CustomEvent('js-message', { detail }))
}

export function addEventMessageListener(messageKey: keyof typeof messageLookup, callback: (payload: any) => void) {
	window.addEventListener('wasm-message', (event) => {
		const messages: Message[] = JSON.parse(event.detail)
		for (let message of messages) {
			if (message.SendEvent?.reg_id === messageLookup[messageKey]) {
				const payload = JSON.parse(message.SendEvent.payload.Json)
				callback(payload)
			}
		}
	})
}

addEventMessageListener('beet_net::replication::common_events::AppReady', (payload) => {
	console.log('YES AppReady', payload)
	sendEventMessage('beet_examples::dialog_panel::OnPlayerMessage', "Hello from JS")
})

declare global {
	interface Window {
		addEventListener(event: 'wasm-message', listener: (event: CustomEvent<string>) => void): void
	}
}