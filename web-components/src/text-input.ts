import '@material/web/button/outlined-button'
import '@material/web/textfield/outlined-text-field.js'
import { MdOutlinedTextField } from '@material/web/textfield/outlined-text-field.js'
import { css, html, LitElement } from 'lit'
import { customElement, query } from 'lit/decorators.js'
import { materialStyles } from './styles'


@customElement('text-input')
export class TextInput extends LitElement {
	@query('md-outlined-text-field') textField!: MdOutlinedTextField

	constructor() {
		super()
	}

	render() {
		return html`
		<div class='container'>
			<md-outlined-text-field placeholder="Send some text" @keyup=${this.onKeyUp}>
  		<md-icon slot="leading-icon">search</md-icon>
			</md-outlined-text-field>
			<md-outlined-button  @click=${this.onSubmit}>
				Submit
			</md-outlined-button>
		</div>
		`
	}


	onKeyUp(event: KeyboardEvent) {
		if (event.key === 'Enter') {
			this.onSubmit()
		}
	}

	onSubmit() {
		const value = this.textField.value
		console.log(value)
		this.textField.value = ""
		// if (this.isRunning)
		// 	this.recognition.stop()
		// else
		// 	this.recognition.start()
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

declare global {
	interface HTMLElementTagNameMap {
		'text-input': TextInput
	}
}