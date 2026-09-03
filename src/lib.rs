//! Bounded benchmark evidence collection and comparability checks.
//!
//! benchproof runs one exact program and argument vector repeatedly. It
//! records observed host conditions and names what was not observed. It does
//! not claim perfect reproducibility or provide a general benchmark runner.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fs;
use std::io::{self, Read};
use std::process::{Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

#[cfg(unix)]
use std::os::unix::process::CommandExt;

pub const SCHEMA_VERSION: u32 = 1;
pub const MAX_REPETITIONS: usize = 100;
pub const MAX_WARMUPS: usize = 100;
pub const MAX_OUTPUT_BYTES: usize = 1_048_576;
pub const DEFAULT_REPETITIONS: usize = 5;
pub const DEFAULT_TIMEOUT_MS: u64 = 60_000;
pub const MAX_TIMEOUT_MS: u64 = 10 * 60 * 1_000;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandSpec {
    pub program: String,
    pub args: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunPolicy {
    pub cache_policy: String,
    pub warmup_iterations: usize,
    pub repetitions: usize,
    pub timeout_ms: u64,
}

impl Default for RunPolicy {
    fn default() -> Self {
        Self {
            cache_policy: "unspecified".to_owned(),
            warmup_iterations: 0,
            repetitions: DEFAULT_REPETITIONS,
            timeout_ms: DEFAULT_TIMEOUT_MS,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OsIdentity {
    pub family: String,
    pub architecture: String,
    pub kernel_release: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CpuIdentity {
    pub model: Option<String>,
    pub logical_cpus: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AffinityObservation {
    pub requested: Option<String>,
    pub requested_cpus: Vec<u32>,
    pub child_setup: String,
    pub observed_process_mask: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HostConditions {
    pub os: OsIdentity,
    pub cpu: CpuIdentity,
    pub affinity: AffinityObservation,
    pub cpu_governor: Option<String>,
    pub current_frequency_khz: Option<u64>,
    pub thermal_celsius: Option<f64>,
    pub background_load_1m: Option<f64>,
    pub filesystem_cache_policy: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Sample {
    pub phase: String,
    pub iteration: usize,
    pub duration_ns: u64,
    pub outcome: String,
    pub exit_code: Option<i32>,
    pub signal: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Summary {
    pub attempted: usize,
    pub successful: usize,
    pub failed: usize,
    pub min_ns: Option<u64>,
    pub max_ns: Option<u64>,
    pub mean_ns: Option<f64>,
    pub median_ns: Option<f64>,
    pub standard_deviation_ns: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BenchmarkRun {
    pub schema_version: u32,
    pub command: CommandSpec,
    pub conditions: HostConditions,
    pub policy: RunPolicy,
    pub warmup_samples: Vec<Sample>,
    pub samples: Vec<Sample>,
    pub summary: Summary,
    pub uncertainty: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComparisonReport {
    pub schema_version: u32,
    pub comparable: bool,
    pub command_match: bool,
    pub policy_match: bool,
    pub condition_differences: Vec<String>,
    pub left_summary: Summary,
    pub right_summary: Summary,
    pub mean_delta_ns: Option<f64>,
    pub mean_delta_percent: Option<f64>,
    pub uncertainty: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct AffinityRequest {
    pub display: String,
    pub cpus: Vec<u32>,
}

impl RunPolicy {
    pub fn bounded(mut self) -> Self {
        self.warmup_iterations = self.warmup_iterations.min(MAX_WARMUPS);
        self.repetitions = self.repetitions.clamp(1, MAX_REPETITIONS);
        self.timeout_ms = self.timeout_ms.min(MAX_TIMEOUT_MS);
        self
    }
}

/// Run a benchmark command with bounded warmups, repetitions, and duration.
pub fn run_benchmark(
    command: &CommandSpec,
    policy: RunPolicy,
    affinity: Option<&AffinityRequest>,
) -> BenchmarkRun {
    let policy = policy.bounded();
    let mut conditions = collect_conditions(&policy.cache_policy, affinity);
    let mut uncertainty = vec![
        "benchmark timing is local monotonic wall-clock evidence, not a proof of reproducibility"
            .to_owned(),
        "scheduler, firmware, background work, and filesystem state can change between runs"
            .to_owned(),
    ];
    if conditions.cpu_governor.is_none() {
        uncertainty.push("CPU governor was not readable".to_owned());
    }
    if conditions.current_frequency_khz.is_none() {
        uncertainty.push("current CPU frequency was not readable".to_owned());
    }
    if conditions.thermal_celsius.is_none() {
        uncertainty.push("thermal telemetry was not readable".to_owned());
    }
    if conditions.background_load_1m.is_none() {
        uncertainty.push("background load was not readable".to_owned());
    }

    let mut warmup_samples = Vec::with_capacity(policy.warmup_iterations);
    for iteration in 0..policy.warmup_iterations {
        let sample = run_once(command, &policy, affinity, "warmup", iteration);
        if sample.outcome != "success" {
            uncertainty.push(format!(
                "warmup iteration {} did not succeed",
                iteration + 1
            ));
        }
        warmup_samples.push(sample);
    }

    let mut samples = Vec::with_capacity(policy.repetitions);
    for iteration in 0..policy.repetitions {
        samples.push(run_once(
            command,
            &policy,
            affinity,
            "measurement",
            iteration,
        ));
    }
    conditions.affinity.child_setup = affinity_setup_summary(&samples, affinity);
    let summary = summarize(&samples);
    if summary.failed > 0 {
        uncertainty.push("one or more measurement invocations failed".to_owned());
    }
    sort_and_deduplicate(&mut uncertainty);
    BenchmarkRun {
        schema_version: SCHEMA_VERSION,
        command: command.clone(),
        conditions,
        policy,
        warmup_samples,
        samples,
        summary,
        uncertainty,
    }
}

/// Compare saved benchmark runs while making condition differences explicit.
pub fn compare_runs(left: &BenchmarkRun, right: &BenchmarkRun) -> ComparisonReport {
    let command_match = left.command == right.command;
    let policy_match = left.policy == right.policy;
    let mut condition_differences = Vec::new();
    if left.conditions.os != right.conditions.os {
        condition_differences.push("os".to_owned());
    }
    if left.conditions.cpu != right.conditions.cpu {
        condition_differences.push("cpu".to_owned());
    }
    if left.conditions.affinity != right.conditions.affinity {
        condition_differences.push("affinity".to_owned());
    }
    if left.conditions.cpu_governor != right.conditions.cpu_governor {
        condition_differences.push("cpu_governor".to_owned());
    }
    if left.conditions.current_frequency_khz != right.conditions.current_frequency_khz {
        condition_differences.push("current_frequency_khz".to_owned());
    }
    if left.conditions.thermal_celsius != right.conditions.thermal_celsius {
        condition_differences.push("thermal_celsius".to_owned());
    }
    if left.conditions.background_load_1m != right.conditions.background_load_1m {
        condition_differences.push("background_load_1m".to_owned());
    }
    if left.conditions.filesystem_cache_policy != right.conditions.filesystem_cache_policy {
        condition_differences.push("filesystem_cache_policy".to_owned());
    }
    let comparable = command_match
        && policy_match
        && condition_differences.is_empty()
        && left.summary.failed == 0
        && right.summary.failed == 0;
    let mean_delta_ns = match (left.summary.mean_ns, right.summary.mean_ns) {
        (Some(left_mean), Some(right_mean)) => Some(right_mean - left_mean),
        _ => None,
    };
    let mean_delta_percent = match (left.summary.mean_ns, mean_delta_ns) {
        (Some(left_mean), Some(delta)) if left_mean != 0.0 => Some(delta / left_mean * 100.0),
        _ => None,
    };
    let mut uncertainty = left.uncertainty.clone();
    uncertainty.extend(right.uncertainty.iter().cloned());
    if !comparable {
        uncertainty.push("runs are not comparable under the recorded command, policy, conditions, or success state".to_owned());
    }
    sort_and_deduplicate(&mut uncertainty);
    ComparisonReport {
        schema_version: SCHEMA_VERSION,
        comparable,
        command_match,
        policy_match,
        condition_differences,
        left_summary: left.summary.clone(),
        right_summary: right.summary.clone(),
        mean_delta_ns,
        mean_delta_percent,
        uncertainty,
    }
}

/// Encode a run or comparison report with a hard output bound.
pub fn report_json<T: Serialize>(report: &T) -> Result<String, String> {
    let rendered = serde_json::to_string(report).map_err(|error| error.to_string())?;
    if rendered.len() > MAX_OUTPUT_BYTES {
        return Err(format!(
            "report is {} bytes, maximum is {} bytes",
            rendered.len(),
            MAX_OUTPUT_BYTES
        ));
    }
    Ok(rendered)
}

/// Decode a saved benchmark run and validate its schema and bounds.
pub fn decode_run(input: &str) -> Result<BenchmarkRun, String> {
    if input.len() > MAX_OUTPUT_BYTES {
        return Err(format!(
            "run report is {} bytes, maximum is {} bytes",
            input.len(),
            MAX_OUTPUT_BYTES
        ));
    }
    let run: BenchmarkRun = serde_json::from_str(input).map_err(|error| error.to_string())?;
    if run.schema_version != SCHEMA_VERSION {
        return Err(format!("unsupported run schema {}", run.schema_version));
    }
    if run.samples.len() > MAX_REPETITIONS || run.warmup_samples.len() > MAX_WARMUPS {
        return Err("run sample count exceeds the configured bound".to_owned());
    }
    Ok(run)
}

/// Render a run without executing its command.
pub fn explain_run(run: &BenchmarkRun) -> String {
    let command = std::iter::once(run.command.program.as_str())
        .chain(run.command.args.iter().map(String::as_str))
        .collect::<Vec<_>>()
        .join(" ");
    let mut lines = vec![
        format!("command: {command}"),
        format!("cache_policy: {}", run.policy.cache_policy),
        format!("warmups: {}", run.policy.warmup_iterations),
        format!("repetitions: {}", run.policy.repetitions),
        format!("successful: {}", run.summary.successful),
        format!("failed: {}", run.summary.failed),
        format!("mean_ns: {:?}", run.summary.mean_ns),
        format!("median_ns: {:?}", run.summary.median_ns),
        "comparable_claim: false".to_owned(),
    ];
    if !run.uncertainty.is_empty() {
        lines.push("uncertainty:".to_owned());
        lines.extend(run.uncertainty.iter().map(|item| format!("- {item}")));
    }
    lines.join("\n")
}

/// Render a comparison without claiming equivalence when conditions differ.
pub fn explain_comparison(report: &ComparisonReport) -> String {
    let mut lines = vec![
        format!("comparable: {}", report.comparable),
        format!("command_match: {}", report.command_match),
        format!("policy_match: {}", report.policy_match),
        format!("mean_delta_ns: {:?}", report.mean_delta_ns),
        format!("mean_delta_percent: {:?}", report.mean_delta_percent),
    ];
    if !report.condition_differences.is_empty() {
        lines.push(format!(
            "condition_differences: {}",
            report.condition_differences.join(",")
        ));
    }
    if !report.uncertainty.is_empty() {
        lines.push("uncertainty:".to_owned());
        lines.extend(report.uncertainty.iter().map(|item| format!("- {item}")));
    }
    lines.join("\n")
}

fn run_once(
    command: &CommandSpec,
    policy: &RunPolicy,
    affinity: Option<&AffinityRequest>,
    phase: &str,
    iteration: usize,
) -> Sample {
    let started = Instant::now();
    let mut process = Command::new(&command.program);
    process.args(&command.args);
    process.stdout(Stdio::null()).stderr(Stdio::null());
    #[cfg(unix)]
    if let Some(affinity) = affinity {
        let cpus = affinity.cpus.clone();
        unsafe {
            process.pre_exec(move || set_child_affinity(&cpus));
        }
    }
    let mut child = match process.spawn() {
        Ok(child) => child,
        Err(_) => {
            return Sample {
                phase: phase.to_owned(),
                iteration,
                duration_ns: elapsed_ns(started),
                outcome: "spawn_error".to_owned(),
                exit_code: None,
                signal: None,
            };
        }
    };
    let deadline = Instant::now()
        .checked_add(Duration::from_millis(policy.timeout_ms))
        .unwrap_or_else(Instant::now);
    let (status, timed_out) = loop {
        match child.try_wait() {
            Ok(Some(status)) => break (Some(status), false),
            Ok(None) if Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(2));
            }
            Ok(None) => {
                let _ = child.kill();
                break (child.wait().ok(), true);
            }
            Err(_) => {
                let _ = child.kill();
                break (child.wait().ok(), false);
            }
        }
    };
    let (exit_code, signal) = status
        .as_ref()
        .map(exit_code_and_signal)
        .unwrap_or((None, None));
    let outcome = if timed_out {
        "timeout"
    } else if status.as_ref().is_some_and(ExitStatus::success) {
        "success"
    } else if signal.is_some() {
        "signaled"
    } else if status.is_some() {
        "nonzero"
    } else {
        "wait_error"
    };
    Sample {
        phase: phase.to_owned(),
        iteration,
        duration_ns: elapsed_ns(started),
        outcome: outcome.to_owned(),
        exit_code,
        signal,
    }
}

fn summarize(samples: &[Sample]) -> Summary {
    let successful = samples
        .iter()
        .filter(|sample| sample.outcome == "success")
        .map(|sample| sample.duration_ns as f64)
        .collect::<Vec<_>>();
    let mut ordered = successful.clone();
    ordered.sort_by(|left, right| left.total_cmp(right));
    let mean_ns = if successful.is_empty() {
        None
    } else {
        Some(successful.iter().sum::<f64>() / successful.len() as f64)
    };
    let median_ns = match ordered.as_slice() {
        [] => None,
        values if values.len() % 2 == 1 => Some(values[values.len() / 2]),
        values => Some((values[values.len() / 2 - 1] + values[values.len() / 2]) / 2.0),
    };
    let standard_deviation_ns = mean_ns.map(|mean| {
        (successful
            .iter()
            .map(|value| (value - mean).powi(2))
            .sum::<f64>()
            / successful.len() as f64)
            .sqrt()
    });
    Summary {
        attempted: samples.len(),
        successful: successful.len(),
        failed: samples.len().saturating_sub(successful.len()),
        min_ns: ordered.first().copied().map(|value| value as u64),
        max_ns: ordered.last().copied().map(|value| value as u64),
        mean_ns,
        median_ns,
        standard_deviation_ns,
    }
}

fn collect_conditions(cache_policy: &str, affinity: Option<&AffinityRequest>) -> HostConditions {
    let kernel_release = read_trimmed("/proc/sys/kernel/osrelease");
    let model = read_first_value("/proc/cpuinfo", "model name");
    let logical_cpus = std::thread::available_parallelism().ok().map(usize::from);
    let observed_process_mask = read_key_value("/proc/self/status", "Cpus_allowed_list");
    HostConditions {
        os: OsIdentity {
            family: std::env::consts::OS.to_owned(),
            architecture: std::env::consts::ARCH.to_owned(),
            kernel_release,
        },
        cpu: CpuIdentity {
            model,
            logical_cpus,
        },
        affinity: AffinityObservation {
            requested: affinity.map(|request| request.display.clone()),
            requested_cpus: affinity
                .map(|request| request.cpus.clone())
                .unwrap_or_default(),
            child_setup: if affinity.is_some() {
                "requested".to_owned()
            } else {
                "not_requested".to_owned()
            },
            observed_process_mask,
        },
        cpu_governor: read_trimmed("/sys/devices/system/cpu/cpu0/cpufreq/scaling_governor"),
        current_frequency_khz: read_trimmed(
            "/sys/devices/system/cpu/cpu0/cpufreq/scaling_cur_freq",
        )
        .and_then(|value| value.parse().ok()),
        thermal_celsius: read_trimmed("/sys/class/thermal/thermal_zone0/temp")
            .and_then(|value| value.parse::<f64>().ok())
            .map(|value| value / 1_000.0),
        background_load_1m: read_trimmed("/proc/loadavg")
            .and_then(|value| value.split_whitespace().next()?.parse::<f64>().ok()),
        filesystem_cache_policy: cache_policy.to_owned(),
    }
}

fn affinity_setup_summary(samples: &[Sample], affinity: Option<&AffinityRequest>) -> String {
    if affinity.is_none() {
        return "not_requested".to_owned();
    }
    if samples.iter().any(|sample| sample.outcome == "spawn_error") {
        "failed_or_unobserved".to_owned()
    } else {
        "requested_for_each_child".to_owned()
    }
}

fn parse_cpu_list(input: &str) -> Result<Vec<u32>, String> {
    let mut cpus = BTreeSet::new();
    for item in input.split(',') {
        let item = item.trim();
        if item.is_empty() {
            return Err("CPU affinity contains an empty item".to_owned());
        }
        if let Some((start, end)) = item.split_once('-') {
            let start = start
                .parse::<u32>()
                .map_err(|_| format!("invalid CPU number {start:?}"))?;
            let end = end
                .parse::<u32>()
                .map_err(|_| format!("invalid CPU number {end:?}"))?;
            if end < start || end.saturating_sub(start) > 1024 {
                return Err(format!("invalid CPU range {item:?}"));
            }
            cpus.extend(start..=end);
        } else {
            cpus.insert(
                item.parse::<u32>()
                    .map_err(|_| format!("invalid CPU number {item:?}"))?,
            );
        }
    }
    if cpus.is_empty() {
        return Err("CPU affinity cannot be empty".to_owned());
    }
    Ok(cpus.into_iter().collect())
}

/// Parse the Linux-style CPU list accepted by the CLI.
pub fn parse_cpu_list_for_cli(input: &str) -> Result<Vec<u32>, String> {
    parse_cpu_list(input)
}

#[cfg(unix)]
fn set_child_affinity(cpus: &[u32]) -> io::Result<()> {
    let mut mask = std::mem::MaybeUninit::<libc::cpu_set_t>::zeroed();
    let mask_bytes = unsafe {
        std::slice::from_raw_parts_mut(
            mask.as_mut_ptr().cast::<u8>(),
            std::mem::size_of::<libc::cpu_set_t>(),
        )
    };
    for cpu in cpus {
        let byte = (*cpu / 8) as usize;
        let bit = *cpu % 8;
        if byte >= mask_bytes.len() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "CPU affinity exceeds Linux cpu_set_t bound",
            ));
        }
        mask_bytes[byte] |= 1 << bit;
    }
    let result = unsafe {
        libc::sched_setaffinity(0, std::mem::size_of::<libc::cpu_set_t>(), mask.as_ptr())
    };
    if result == -1 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(unix)]
fn exit_code_and_signal(status: &ExitStatus) -> (Option<i32>, Option<i32>) {
    use std::os::unix::process::ExitStatusExt;
    (status.code(), status.signal())
}

#[cfg(not(unix))]
fn exit_code_and_signal(status: &ExitStatus) -> (Option<i32>, Option<i32>) {
    (status.code(), None)
}

fn elapsed_ns(started: Instant) -> u64 {
    started.elapsed().as_nanos().min(u64::MAX as u128) as u64
}

fn read_trimmed(path: &str) -> Option<String> {
    let mut file = fs::File::open(path).ok()?;
    let mut bytes = Vec::new();
    file.by_ref().take(16 * 1024).read_to_end(&mut bytes).ok()?;
    String::from_utf8(bytes)
        .ok()
        .map(|value| value.trim().to_owned())
}

fn read_first_value(path: &str, key: &str) -> Option<String> {
    let content = read_trimmed(path)?;
    content.lines().find_map(|line| {
        let (name, value) = line.split_once(':')?;
        (name.trim() == key).then(|| value.trim().to_owned())
    })
}

fn read_key_value(path: &str, key: &str) -> Option<String> {
    let content = read_trimmed(path)?;
    content.lines().find_map(|line| {
        let (name, value) = line.split_once(':')?;
        (name.trim() == key).then(|| value.trim().to_owned())
    })
}

fn sort_and_deduplicate(items: &mut Vec<String>) {
    items.sort();
    items.dedup();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> CommandSpec {
        CommandSpec {
            program: "/bin/sh".to_owned(),
            args: vec!["-c".to_owned(), "printf fixture".to_owned()],
        }
    }

    #[test]
    fn deterministic_fixture_records_samples_and_summary() {
        let run = run_benchmark(
            &fixture(),
            RunPolicy {
                warmup_iterations: 1,
                repetitions: 3,
                ..RunPolicy::default()
            },
            None,
        );
        assert_eq!(run.warmup_samples.len(), 1);
        assert_eq!(run.samples.len(), 3);
        assert_eq!(run.summary.attempted, 3);
        assert_eq!(run.summary.successful, 3);
        assert!(run.summary.mean_ns.is_some());
    }

    #[test]
    fn failed_command_is_recorded() {
        let command = CommandSpec {
            program: "/bin/sh".to_owned(),
            args: vec!["-c".to_owned(), "exit 3".to_owned()],
        };
        let run = run_benchmark(&command, RunPolicy::default(), None);
        assert_eq!(run.summary.failed, DEFAULT_REPETITIONS);
        assert_eq!(run.samples[0].outcome, "nonzero");
    }

    #[test]
    fn noisy_fixture_has_nonzero_variance_or_uncertainty() {
        let command = CommandSpec {
            program: "/bin/sh".to_owned(),
            args: vec!["-c".to_owned(), "sleep 0.001".to_owned()],
        };
        let run = run_benchmark(
            &command,
            RunPolicy {
                repetitions: 3,
                ..RunPolicy::default()
            },
            None,
        );
        assert_eq!(run.summary.successful, 3);
        assert!(run.summary.standard_deviation_ns.unwrap_or(0.0) >= 0.0);
        assert!(run.uncertainty.iter().any(|item| item.contains("timing")));
    }

    #[test]
    fn missing_and_unreadable_metadata_stays_unobserved() {
        assert_eq!(read_trimmed("/no/such/benchproof-metadata"), None);
        let directory = std::env::temp_dir();
        assert_eq!(read_trimmed(directory.to_str().unwrap()), None);
    }

    #[test]
    fn affinity_parser_is_deterministic_and_bounded() {
        assert_eq!(parse_cpu_list("3,1-2").unwrap(), vec![1, 2, 3]);
        assert!(parse_cpu_list("2-1").is_err());
        assert!(parse_cpu_list("").is_err());
    }

    #[test]
    fn different_conditions_are_not_comparable() {
        let left = run_benchmark(&fixture(), RunPolicy::default(), None);
        let right = run_benchmark(
            &fixture(),
            RunPolicy {
                cache_policy: "warm".to_owned(),
                ..RunPolicy::default()
            },
            None,
        );
        let report = compare_runs(&left, &right);
        assert!(!report.comparable);
        assert!(
            report
                .condition_differences
                .contains(&"filesystem_cache_policy".to_owned())
        );
    }

    #[test]
    fn schema_round_trip_is_stable_and_bounded() {
        let run = run_benchmark(&fixture(), RunPolicy::default(), None);
        let json = report_json(&run).unwrap();
        let decoded = decode_run(&json).unwrap();
        assert_eq!(decoded.schema_version, SCHEMA_VERSION);
        assert_eq!(decoded.command, run.command);
    }

    #[test]
    fn report_explanation_names_uncertainty() {
        let run = run_benchmark(&fixture(), RunPolicy::default(), None);
        assert!(explain_run(&run).contains("comparable_claim: false"));
        let report = compare_runs(&run, &run);
        assert!(explain_comparison(&report).contains("comparable:"));
    }
}
