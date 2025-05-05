# Beet RSX Combinator

JSX-like parser combinator for Rust.

This crate is a fork of Victor Porof's [rsx-parser](https://github.com/victorporof/rsx-parser)

## Purpose
This is an experimental parser for JSX-like code in Rust. The long term goal might be to build "something like React" in Rust, but this can mean a number of things, from a direct port with 100% API compatibility to a completely different product. A JSX-like parser is a good and simple place to start experimenting from.

## RSX vs. JSX
The [JSX spec](http://facebook.github.io/jsx) is, although a draft, presumably stable. Syntax extension equivalents can be found for Rust, which is the main scope of this experiment.

Example, inspired from the JSX spec website linked above:

```jsx
const FunDropdown = (props) =>
  <Dropdown show={props.visible}>
    A dropdown list
    <Menu
      icon={props.menu.icon}
      onHide={(e) => console.log(e)}
      onShow={(e) => console.log(e)}
    >
      <MenuItem>Do Something</MenuItem>
      {
        shouldDoSomethingFun()
          ? <MenuItem>Do Something Fun!</MenuItem>
          : <MenuItem>Do Something Else</MenuItem>
      }
    </Menu>
  </Dropdown>;
```

An equivalent interpretation of JSX in Rust, using compiler plugins, looks this:

```rust
fn fun_dropdown(props: Props) -> RSXElement {
  rsx! {
    <Dropdown show={props.visible}>
      A dropdown list
      <Menu
        icon={props.menu.icon}
        onHide={|e: Event| println!("{:?}", e)}
        onShow={|e: Event| println!("{:?}", e)}
      >
        <MenuItem>Do Something</MenuItem>
        {
          if should_do_something_fun() {
            <MenuItem>Do Something Fun!</MenuItem>
          } else {
            <MenuItem>Do Something Else</MenuItem>
          }
        }
      </Menu>
    </Dropdown>
  }
}
```

## Supported grammar
All of the [JSX official grammar](http://facebook.github.io/jsx) is supported. In the case of handling arbitrary Rust code inside RSX, the treatment is similar: JSX can contain JavaScript "code blocks" delimited by curly braces (specifically "assignment expressions"), in clearly defined locations such as attribute values, children contents etc. Rust expressions provide sufficient equivalence.

### PrimaryExpression
- [X] JSXElement

### Elements

#### JSXElement
- [X] JSXSelfClosingElement
- [X] JSXOpeningElement JSXChildren? JSXClosingElement

#### JSXSelfClosingElement
- [X] `<` JSXElementName JSXAttributes? `/` `>`

#### JSXOpeningElement
- [X] `<` JSXElementName JSXAttributes? `>`

#### JSXClosingElement
- [X] `<` `/` JSXElementName `>`

#### JSXElementName
- [X] JSXIdentifier
- [X] JSXNamedspacedName
- [X] JSXMemberExpression

#### JSXIdentifier
- [X] IdentifierStart
- [X] JSXIdentifier IdentifierPart
- [X] JSXIdentifier `-`

#### JSXNamespacedName
- [X] JSXIdentifier `:` JSXIdentifier

#### JSXMemberExpression
- [X] JSXIdentifier `.` JSXIdentifier
- [X] JSXMemberExpression `.` JSXIdentifier

### Attributes

#### JSXAttributes
- [X] JSXSpreadAttribute JSXAttributes?
- [X] JSXAttribute JSXAttributes?

#### JSXSpreadAttribute
- [X] `{` ... AssignmentExpression `}`

#### JSXAttribute
- [X] JSXAttributeName `=` JSXAttributeValue

#### JSXAttributeName
- [X] JSXIdentifier
- [X] JSXNamespacedName

#### JSXAttributeValue
- [X] `"` JSXDoubleStringCharacters? `"`
- [X] `'` JSXSingleStringCharacters? `'`
- [X] `{` AssignmentExpression `}`
- [X] JSXElement

#### JSXDoubleStringCharacters
- [X] JSXDoubleStringCharacter JSXDoubleStringCharacters?

#### JSXDoubleStringCharacter
- [X] SourceCharacter *but not* `"`

#### JSXSingleStringCharacters
- [X] JSXSingleStringCharacter JSXSingleStringCharacters?

#### JSXSingleStringCharacter
- [X] SourceCharacter *but not* `'`

### Children

#### JSXChildren
- [X] JSXChild JSXChildren?

#### JSXChild
- [X] JSXText
- [X] JSXElement
- [X] `{` AssignmentExpression? `}`

#### JSXText
- [X] JSXTextCharacter JSXText?

#### JSXTextCharacter
- [X] SourceCharacter *but not one of* `{`, `<`, `>` *or* `}`



## License

Copyright 2016 Mozilla
Licensed under the Apache License, Version 2.0 (the "License"); you may not use
this file except in compliance with the License. You may obtain a copy of the
License at http://www.apache.org/licenses/LICENSE-2.0
Unless required by applicable law or agreed to in writing, software distributed
under the License is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR
CONDITIONS OF ANY KIND, either express or implied. See the License for the
specific language governing permissions and limitations under the License.
