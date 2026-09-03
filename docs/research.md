# Research notes

Research date: 2026-09-03.

Primary sources:

- Rust's [`Instant`](https://doc.rust-lang.org/std/time/struct.Instant.html)
  documents a monotonic clock abstraction for measuring elapsed intervals.
- Rust's [`Command`](https://doc.rust-lang.org/std/process/struct.Command.html)
  documents direct program and argument construction.
- Rust's [`Child`](https://doc.rust-lang.org/std/process/struct.Child.html)
  documents spawned-child waiting, polling, and termination methods.
- Rust's [`ExitStatus`](https://doc.rust-lang.org/std/process/struct.ExitStatus.html)
  documents portable success and exit-code inspection.
- Unix [`ExitStatusExt`](https://doc.rust-lang.org/std/os/unix/process/trait.ExitStatusExt.html)
  documents signal access for Unix exit statuses.
- The JSON [`RFC 8259`](https://www.rfc-editor.org/rfc/rfc8259) defines the
  interchange format used for run and comparison reports.

Distribution signal: Cargo package metadata and a standalone CLI repository
are prepared for crates.io. Package availability or download counts are
distribution signals, not evidence of willingness to pay.

Evidence grade: the cited Rust documentation supports the direct process and
monotonic timing APIs. Host probes, comparability rules, statistics, output
bounds, and uncertainty wording are design inferences validated by local
tests. They are not universal controls over hardware or operating-system
state.

Rejected alternatives include a cache flusher, scheduler, or hyperfine-style
feature expansion, which would add control or scope beyond the evidence
question. Decision: keep the release to bounded local runs, condition records,
raw samples, comparison warnings, and explicit uncertainty.
