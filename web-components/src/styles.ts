import { css } from "lit"



export const materialStyles = css`
:host {
      --md-ref-typeface-brand: 'Open Sans';
	    --md-ref-typeface-plain: system-ui;
      font-family: 'Roboto', 'Arial', 'Helvetica', sans-serif;
    }

    md-filled-button,md-outlined-button {
  /* mdbook breaks size for some reason */
  font-size:0.9em;

}
`