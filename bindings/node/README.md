# rustille (Node.js)

Turn pixels into Braille.

```js
import { renderFile, renderRgba, Canvas } from "rustille";

console.log(renderFile("cat.png", { width: 80 }));
```

Native Node-API bindings over the [Rustille](https://github.com/a269ch/rustille)
Rust core. Prebuilt binaries ship for Linux (gnu and musl), macOS and Windows on
x64 and arm64, so installing does not require a Rust toolchain.

See the main README for the full documentation.
