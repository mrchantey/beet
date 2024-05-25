import '@material/web/button/filled-button.js'
import '@material/web/icon/icon.js'
import '@material/web/progress/linear-progress.js'

import { LitElement, css, html } from 'lit'
import { customElement, property } from 'lit/decorators.js'
import { materialStyles } from './styles'

type LoadState = 'loading' | 'loaded' | 'running' | 'error'


/// Canvas loader
@customElement('beet-loading-canvas')
export class BeetCanvas extends LitElement {
  /// Wait for `finshOnLoad` to be called manually
  @property({ type: Boolean, attribute: 'custom-load' })
  customLoad: boolean = false

  /// Display a start button on load
  @property({ type: Boolean, attribute: 'require-interaction' })
  requireInteraction: boolean = false

  /// The id of the canvas that will be created
  @property({ type: String, attribute: 'canvas-id' })
  canvasId: string = 'beet-canvas'

  /// The source file
  @property({ type: String })
  src: string | null = null

  /// The load state of the canvas
  @property({ type: String })
  loadState: LoadState = 'loading'

  errorMessage: string | null = null

  connectedCallback(): void {
    super.connectedCallback()
    this.innerHTML += `<canvas id='${this.canvasId}' slot='canvas-slot'></canvas>`
    this.tryLoadSrc()
  }

  async tryLoadSrc() {
    if (!this.src) {
      return
    }

    const base = this.src.startsWith('http') ? undefined : window.location.href
    const src = new URL(this.src, base)
    const module = await import(/* @vite-ignore */src.href)
    await module.default().catch((error: Error) => {
      if (error.message.startsWith("Using exceptions for control flow,")) {
        return
      } else {
        this.errorMessage = error.message
        this.loadState = 'error'
        this.requestUpdate()
        throw error
      }
    })
    if (this.customLoad == false) {
      this.finishedLoading()
    }
  }


  render() {

    let showOverlay = this.loadState !== 'running'
    let showProgress = this.loadState === 'loading'
    let showButton = this.requireInteraction && this.loadState === 'loaded'
    let showError = this.errorMessage !== null

    return html`
      <div class='container'>
        <slot name="canvas-slot"></slot>
        <div class='overlay' ?hidden=${!showOverlay}>
          <div class='progress-overlay' ?hidden=${!showProgress}>
            <md-linear-progress indeterminate></md-circular-progress>
          </div>
          <div class='button-container' ?hidden=${!showButton}>
            <md-filled-button @click=${this.startRunning} part="button">
              Start
            </md-filled-button>
          </div>
          <div class='error-message' ?hidden=${!showError}>Error: ${this.errorMessage}</div>
      </div>
      </div>
    `
  }

  finishedLoading() {
    if (this.loadState !== 'loading') {
      return
    }
    if (this.requireInteraction) {
      this.loadState = 'loaded'
      this.requestUpdate()
    } else {
      this.startRunning()
    }
  }

  startRunning() {
    this.loadState = 'running'
    this.requestUpdate()
    this.dispatchEvent(new CustomEvent('start'))
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
    'beet-loading-canvas': BeetCanvas
  }
}