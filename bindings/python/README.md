# rustille (Python)

Turn pixels into Braille.

```python
import rustille

print(rustille.render_file("cat.png", width=80))
```

This package is a thin binding over the [Rustille](https://github.com/a269ch/rustille)
Rust core; see the main README for the full documentation.
