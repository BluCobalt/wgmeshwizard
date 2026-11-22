# Translations

Each `<locale>.yml` file is loaded by `rust-i18n` at compile time. `en.yml` is
the source catalogue: its keys are stable semantic identifiers such as
`ui.canvas.no_hosts_yet`, and its values are the English strings used in the UI.

To add a language:

1. Copy `en.yml` to `<locale>.yml` (for example `de.yml` or `pt-BR.yml`).
2. Keep every key and translate only the values.
3. Rebuild. The language appears automatically in **Settings → General →
   Language** because the list is read from `rust_i18n::available_locales!()`.

Placeholders use `%{name}` syntax and must match the names passed from the Rust
call site, for example:

```yaml
"ui.add_host_near.added_name_and_centered_the_canvas": "Added %{name} and centered the canvas on it."
```

Missing translations fall back to the English value from `en.yml`.
