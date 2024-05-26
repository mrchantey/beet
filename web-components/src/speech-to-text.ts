import '@material/web/switch/switch.js'
import { MdSwitch } from '@material/web/switch/switch.js'
import { css, html, LitElement } from 'lit'
import { customElement, property } from 'lit/decorators.js'
import { materialStyles } from './styles'

import style from './style/foobar.module.css'

console.dir(style)
console.log("val:", JSON.stringify(style))


@customElement('speech-to-text')
export class SpeechToText extends LitElement {
	recognition: SpeechRecognition
	currentIndex: number = 0
	isRunning: boolean = false

	canvasId: string = 'beet-canvas'

	@property({ type: Boolean, attribute: 'debug' })
	debug: boolean = false

	constructor() {
		super()
		const recognition = getSpeechRecognition()
		recognition.continuous = true
		recognition.lang = "en-US"
		recognition.interimResults = false
		recognition.onresult = (event) => {
			for (let i = this.currentIndex; i < event.results.length; i++) {
				let value = event.results[i][0]?.transcript || ""
				value = value.trim()
				if (value.length === 0)
					continue
				this.dispatchEvent(new CustomEvent('speech', { detail: value }))
				if (this.debug)
					console.log("speech: ", value)
			}
			this.currentIndex = event.results.length
		}
		recognition.onstart = () => {
			this.isRunning = true
			this.requestUpdate()
		}
		recognition.onend = () => {
			this.isRunning = false
			this.requestUpdate()
		}
		this.recognition = recognition
	}

	render() {
		let text = this.isRunning ? "Running" : ""
		return html`
		<div class='row-container'>
			<span>Speech to Text: ${text}</span>
			<md-switch @change=${this.onClick}>
			</md-switch>
		</div>
		`
	}

	onClick(e: Event) {
		let checked = (e.target as MdSwitch).selected
		if (checked)
			this.recognition.start()
		else
			this.recognition.stop()
	}

	static styles = [materialStyles, css`
	.container{
		display: flex;
		flex-direction: row;
		align-items: center;
		gap: 10px;
	}
	`]
}

function getSpeechRecognition(): SpeechRecognition {
	let val = window.SpeechRecognition || window.webkitSpeechRecognition
	return new val()
}

declare global {
	interface HTMLElementTagNameMap {
		'speech-to-text': SpeechToText
	}
}