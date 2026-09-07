# Contributing

Thank you for considering a contribution. This project is in **alpha**; APIs and on-disk formats may still change.

## Requirements

- Rust **1.85** or newer (MSRV)
- Nightly `rustfmt` for formatting (`rustup component add rustfmt --toolchain nightly`)

## Workflow

1. Fork and open a pull request against `main`. Direct pushes to `main` are blocked.
2. Keep changes focused. Match existing module layout and naming.
3. Add or update tests for behavior you change. Integration tests live in `tests/`; unit tests sit next to the code in `#[cfg(test)]` modules.

## Checks (same as CI)

```bash
cargo check --all-targets --locked
cargo test --locked
cargo clippy --all-targets --locked -- -D warnings
rustup run nightly cargo fmt --all -- --check
cargo deny check
```

`cargo deny` uses `deny.toml`. New dependencies must use a license already on the allow list (this crate itself is **AGPL-3.0-only**).

## License

By contributing, you agree that your contributions are licensed under the GNU Affero General Public License v3.0 only (`AGPL-3.0-only`), as in [LICENSE](LICENSE).
