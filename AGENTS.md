# Local operating contract

This repository is a focused Rust project. The product brief and CI workflow
are authoritative. Read `docs/design.md` for the benchmark evidence model,
`docs/limits.md` for boundaries, and `docs/release.md` for release evidence.

## Commands

- Build: `cargo build --locked`
- Test: `cargo test --all-targets --locked`
- Format check: `cargo fmt --all -- --check`
- Lint: `cargo clippy --all-targets --all-features --locked -- -D warnings`
- Documentation: `RUSTDOCFLAGS=-Dwarnings cargo doc --no-deps --locked`
- Package check: `cargo package --locked`
- CLI smoke test: use the commands in `docs/release.md`

## Scope

Keep changes limited to bounded benchmark evidence and comparability reports.
Do not add a hyperfine replacement, statistical promise, hosted service,
cache flusher, scheduler, or untested platform claim.

## Operating loop

1. Plan the change and define a measurable success condition.
2. Make only scoped edits.
3. Read back every changed file.
4. Run relevant validation commands and record exact results.
5. Review the diff before committing or pushing.

## Safety and release

The harness executes the exact program and argument vector supplied by the
operator, with bounded repetition and timeout policies. It does not claim
perfect reproducibility. Reports can contain command arguments and host
identity. Never put credentials, private paths, or private QA artifacts in
source or tracked files. Use `docs/release.md`; release requires clean local
and hosted gates, independent artifact verification, and a truthful limits
record.
