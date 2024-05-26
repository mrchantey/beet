import { sendPlayerMessage } from "../beet/message"


export class PhraseDetector {
	recognition: SpeechRecognition
	currentIndex: number = 0
	isRunning: boolean = false
	debug: boolean = false

	constructor() {
		const recognition = new (window.SpeechRecognition || window.webkitSpeechRecognition)()
		recognition.continuous = true
		recognition.lang = "en-US"
		recognition.interimResults = false
		recognition.onresult = (event) => {
			for (let i = this.currentIndex; i < event.results.length; i++) {
				let value = event.results[i][0]?.transcript || ""
				value = value.trim()
				if (value.length === 0)
					continue
				sendPlayerMessage(value)
				if (this.debug)
					console.log("speech: ", value)
			}
			this.currentIndex = event.results.length
		}
		recognition.onstart = () => {
			this.isRunning = true
		}
		recognition.onend = () => {
			this.isRunning = false
		}
		this.recognition = recognition
	}

	start() {
		this.recognition.start()
	}
	stop() {
		this.recognition.stop()
	}
}