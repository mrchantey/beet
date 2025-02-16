# vscode-rsx-highlighting

## Overview
This project is a Visual Studio Code extension that provides syntax highlighting and IntelliSense for `rsx!` macros in Rust, enabling a better development experience when working with HTML and CSS within Rust code.

## Features
- Syntax highlighting for HTML and CSS elements inside `rsx!` macros.
- IntelliSense support for HTML and CSS attributes and elements.
- Hover information for HTML and CSS elements.
- Format on save with `leptosfmt`

## Local development
1. Clone the repository:
   ```
   git clone https://github.com/mrchantey/beet.git
   ```
2. Navigate to the project directory:
   ```
   cd beet/crates/beet-vscode
   ```
3. Install the dependencies:
   ```
   npm install
   ```
4. Open the project in Visual Studio Code:
   ```
   code .
   ```
5. Press `F5` to run the extension in a new Extension Development Host window.

## Usage
- Open a Rust file containing `rsx!` macros.
- Start typing HTML or CSS within the `rsx!` macro to see IntelliSense suggestions.
- Hover over HTML or CSS elements to view additional information.

## Contributing
Contributions are welcome! Please open an issue or submit a pull request for any enhancements or bug fixes.

## License
This project is licensed under the MIT License. See the LICENSE file for details.