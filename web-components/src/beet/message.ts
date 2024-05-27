interface Message {
	SendEvent?: {
		reg_id: number
		payload: {
			Json: string
		}
	}
}

const messageLookup = {
	"AppReady": 0,
	"OnPlayerMessage": 1
}

export function sendPlayerMessage(message: string) {
	sendEventMessage("OnPlayerMessage", message)
}
export function sendStartGameMessage() {
	console.warn('todo: send start game message')
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
	console.log('js-message:\n', detail)
	window.dispatchEvent(new CustomEvent('js-message', { detail }))
}


export function removeMessageListener(callback: () => void) {
	window.removeEventListener('wasm-message', callback)
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

declare global {
	interface Window {
		addEventListener(event: 'wasm-message', listener: (event: CustomEvent<string>) => void): void
	}
}