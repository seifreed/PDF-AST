# PDF-AST Dynamic Plugin Example

Build the plugin as a shared library:

```
cargo build --manifest-path examples/dynamic_plugin/Cargo.toml --release
```

Load it with the plugin loader configuration:

```
{
  "plugins": [
    {
      "name": "example_metadata",
      "type": "dynamic",
      "path": "examples/dynamic_plugin/target/release/libpdf_ast_dynamic_plugin_example.dylib"
    }
  ]
}
```

On Linux the library name ends with `.so` and on Windows `.dll`.
