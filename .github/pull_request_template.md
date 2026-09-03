## Scope

- [ ] The change stays within bounded benchmark evidence and comparability.
- [ ] No claim of perfect reproducibility or hyperfine replacement was added.
- [ ] No hosted service, private environment value, or generated fuzz corpus is
      included.

## Verification

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo check --all-targets --locked`
- [ ] `cargo clippy --all-targets --all-features --locked -- -D warnings`
- [ ] `cargo test --all-targets --locked`
- [ ] `RUSTDOCFLAGS=-Dwarnings cargo doc --no-deps --locked`
- [ ] `cargo package --locked`
- [ ] `cargo audit`
