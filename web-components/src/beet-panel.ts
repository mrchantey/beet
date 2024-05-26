import './beet-loading-canvas'
import './speech-to-text'
import './text-input'

import { LitElement, css } from 'lit'
import { customElement } from 'lit/decorators.js'
import { materialStyles } from './styles'


/// Canvas loader
@customElement('beet-panel')
export class BeetPanel extends LitElement {

	connectedCallback(): void {
		super.connectedCallback()
	}

	render() {

		// return html`
		//   <div class='container'>
		//     <slot name="canvas-slot"></slot>
		//     <div class='overlay' ?hidden=${!showOverlay}>
		//       <div class='progress-overlay' ?hidden=${!showProgress}>
		//         <md-linear-progress indeterminate></md-circular-progress>
		//       </div>
		//       <div class='button-container' ?hidden=${!showButton}>
		//         <md-filled-button @click=${this.startRunning} part="button">
		//           Start
		//         </md-filled-button>
		//       </div>
		//       <div class='error-message' ?hidden=${!showError}>Error: ${this.errorMessage}</div>
		//   </div>
		//   </div>
		// `
	}


	static styles = [materialStyles, css`
    
    .container{
      position: relative;
      width: 100%;
      height: 100%;
      /* min-height: 400px; */
    }

    ::slotted(canvas) {
      width: 100%;
      height: 100%;
      border: none;
      outline: none;
    }
    
    .overlay{
      display: flex;
      justify-content: center;
      align-items: center;
      position: absolute;
      flex-direction: column;
      top: 0;
      left: 0;
	    right: 0;
	    bottom: 0;
    }
        
    .error-message{
      background-color: white;
      color: red;
    }
    
    .progress-overlay{
      width:80%;
    }
  `]
}

declare global {
	interface HTMLElementTagNameMap {
		'beet-panel': BeetPanel
	}
}