# Product brief

benchproof records enough local execution conditions to tell whether two
benchmark runs are meaningfully comparable.

Target users are maintainers and performance investigators who need evidence
around a local command's timing without a promise that the machine is fully
controlled or that the result is perfectly reproducible.

The first commands are:

```text
benchproof run -- ./benchmark
benchproof compare run-a.json run-b.json
benchproof inspect run.json
```

The switching wedge is a small versioned record of command, host conditions,
warmup and repetition policy, raw samples, summary statistics, and named
uncertainty. It makes a comparison's missing evidence visible.

Evidence and inference are separate. Rust process and timing documentation
linked in [`docs/research.md`](research.md) supports the documented APIs. The
condition fields, comparability policy, bounds, and uncertainty language are
design choices verified by local tests, not guarantees about every host.

Non-goals are perfect reproducibility, benchmark correctness proofs,
statistical significance, cache control, scheduling, profiling, and hosted
execution.
