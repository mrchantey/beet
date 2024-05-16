# Fetch

- Please wait for status to change to `Idle` before issuing commands.

<input id="prompt" value="fetch my weapon!">
<button id="submit">Submit</button>
<br/>
<br/>
<iframe src="https://storage.googleapis.com/beet-examples/fetch/index.html" allowTransparency="true"></iframe>

> **Web troubleshooting**
> Sometimes retrieving the model returns a `403 Header Too Large`, I'm currently looking into a solution.

<script>
function send(){
	let input = document.getElementById('prompt');
	if (input.value == "")
		return;
	const iframe = document.querySelector('iframe').contentWindow;
	iframe.postMessage(input.value, '*');
	input.value = "";
}
document.getElementById('submit').addEventListener('click', send);
document.getElementById('prompt').addEventListener('keyup', function(event) {
	if (event.key === 'Enter')
		send();
});
</script>

```rust
{{#include ../../../examples/fetch.rs}}
```