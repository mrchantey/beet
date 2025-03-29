
This chart contains the build steps for a typical Beet App.

```mermaid
graph TD;
    A(("Collection Definitions")) --> B["Pre Build Native"];
    B --> C(("Collection Codegen"));
    C --> D["Build Native"];
    E(("Src Code")) --> D;
    E --> F["Build Wasm"];
    D --> G(("Native Binary"));
    G --> H["Run Server"];
    G --> I["Export Static"];
    I --> J(("HTML"));
    J --> H;
    I --> K(("Islands Serde"));
    K --> L["Pre Build Wasm"];
		L --> M(("Wasm Entry Codegen"));
    M --> F;
    F --> N(("Wasm Binary"));
    N --> H;
```
