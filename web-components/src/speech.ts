


typeof SpeechRecognitionResult
const a = window


const grammar =
	"#JSGF V1.0; grammar colors; public <color> = aqua | azure | beige | bisque | black | blue | brown | chocolate | coral | crimson | cyan | fuchsia | ghostwhite | gold | goldenrod | gray | green | indigo | ivory | khaki | lavender | lime | linen | magenta | maroon | moccasin | navy | olive | orange | orchid | peru | pink | plum | purple | red | salmon | sienna | silver | snow | tan | teal | thistle | tomato | turquoise | violet | white | yellow ;"
const recognition = new SpeechRecognition()
const speechRecognitionList = new SpeechGrammarList()
speechRecognitionList.addFromString(grammar, 1)
recognition.grammars = speechRecognitionList
recognition.continuous = false
recognition.lang = "en-US"
recognition.interimResults = false
recognition.maxAlternatives = 1

const diagnostic = document.querySelector(".output")
const bg = document.querySelector("html")


window.addEventListener('click', () => {
	console.log('hi')
	console.log("Ready to receive a color command.")
	recognition.start()
})

// document.body.onclick = () => {
// };

recognition.onresult = (event) => {
	const color = event.results[0][0].transcript
	console.log('result: ', color)
	if (diagnostic) diagnostic.textContent = `Result received: ${color}`
	if (bg) bg.style.backgroundColor = color
};

