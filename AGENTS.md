# Repository instructions

- Use Conventional Commits 1.0.0 for every commit, including the initial commit: https://www.conventionalcommits.org/en/v1.0.0/.
- Write commit messages, code, comments, and documentation in English. Czech belongs in the translation catalog and relevant test fixtures.
- Keep credentials and live task data out of commits and test fixtures.
- Run `cargo fmt --check`, `cargo test --locked`, and `cargo clippy --locked --all-targets -- -D warnings` before pushing code changes.
