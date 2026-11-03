# Contributing

Small, focused changes are easiest to review. Open an issue first for a substantial change in behavior or platform support; straightforward bug fixes and translation corrections can go directly to a pull request.

## Development

Follow the platform setup in the [README](README.md), then run:

```sh
cargo run --locked
cargo fmt --check
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
```

Use `cargo test --locked --no-default-features` for the API, locale, and summary tests without building the desktop UI. Native UI changes should also be checked in a real desktop session. CI builds macOS Apple Silicon, macOS Intel, and Windows.

Clippy is the Rust linter. Its project rules reject leftover `dbg!`, `todo!`, and `unimplemented!` calls; CI treats every Clippy warning as an error. Rustfmt checks formatting separately. Both tools are installed by the pinned toolchain.

## Code changes

- Keep network and credential access on the worker thread.
- Preserve read-before-write and read-after-write timer checks. Do not automatically retry task creation or timer mutations.
- Add a regression test for a behavioral bug. Prefer observable outcomes and local HTTP fixtures over tests that assert implementation details.
- Keep code, identifiers, comments, documentation, and test names in English.
- Do not add a server, browser engine, telemetry, or a new dependency without explaining why the existing architecture cannot support the change.
- Never include real API tokens, private task data, or local preference files in patches, screenshots, or logs.

## Translations

App text lives in `src/locale.rs`. Add a typed key with both English and Czech text, and render through the selected `Language`. User-supplied task names and workspace names stay unchanged.

Use structured `Message` errors so an error can be rendered in the current language even after it was created. Do not translate by matching English error strings. Placeholder positions must agree between locales; catalog tests check coverage and interpolation. Keep framework control translations aligned with the components the app actually uses.

## Pull requests

Explain the concrete problem, the resulting behavior, and how you verified it. Include the OS and architecture for native UI changes. If you cannot test a platform, state that explicitly instead of claiming it works.
