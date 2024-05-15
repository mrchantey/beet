# Fetch

<iframe src="https://storage.googleapis.com/beet-examples/fetch/index.html"></iframe>

<input id="prompt" value="fetch my weapon!">
<button id="submit">Submit</button>

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