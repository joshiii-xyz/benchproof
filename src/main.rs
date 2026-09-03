use benchproof::{
    AffinityRequest, CommandSpec, DEFAULT_REPETITIONS, DEFAULT_TIMEOUT_MS, RunPolicy, compare_runs,
    decode_run, explain_comparison, explain_run, parse_cpu_list_for_cli, report_json,
    run_benchmark,
};
use clap::{Parser, Subcommand, ValueEnum};
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Debug, Clone, Copy, ValueEnum)]
enum OutputFormat {
    Json,
    Text,
}

#[derive(Debug, Parser)]
#[command(
    name = "benchproof",
    version,
    about = "Record benchmark conditions and state comparability limits"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Run one exact benchmark command.
    Run {
        #[arg(long, value_enum, default_value_t = OutputFormat::Json)]
        format: OutputFormat,
        #[arg(long, default_value = "unspecified")]
        cache_policy: String,
        #[arg(long, default_value_t = 0)]
        warmup: usize,
        #[arg(long, default_value_t = DEFAULT_REPETITIONS)]
        repetitions: usize,
        #[arg(long, default_value_t = DEFAULT_TIMEOUT_MS)]
        timeout_ms: u64,
        #[arg(long, value_name = "CPU_LIST")]
        affinity: Option<String>,
        #[arg(last = true, required = true)]
        command: Vec<String>,
    },
    /// Compare two saved benchmark reports.
    Compare {
        left: PathBuf,
        right: PathBuf,
        #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
        format: OutputFormat,
    },
    /// Inspect one saved benchmark report.
    Inspect {
        run: PathBuf,
        #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
        format: OutputFormat,
    },
}

fn main() -> ExitCode {
    match execute(Cli::parse()) {
        Ok(code) => ExitCode::from(code),
        Err(error) => {
            eprintln!("benchproof: {error}");
            ExitCode::from(2)
        }
    }
}

fn execute(cli: Cli) -> Result<u8, Box<dyn std::error::Error>> {
    match cli.command {
        Commands::Run {
            format,
            cache_policy,
            warmup,
            repetitions,
            timeout_ms,
            affinity,
            command,
        } => {
            if command.is_empty() {
                return Err("a command is required after --".into());
            }
            let affinity = affinity
                .as_deref()
                .map(parse_cpu_list_for_cli)
                .transpose()?;
            let request = affinity.as_ref().map(|cpus| AffinityRequest {
                display: command_affinity_display(cpus),
                cpus: cpus.clone(),
            });
            let spec = CommandSpec {
                program: command[0].clone(),
                args: command[1..].to_vec(),
            };
            let run = run_benchmark(
                &spec,
                RunPolicy {
                    cache_policy,
                    warmup_iterations: warmup,
                    repetitions,
                    timeout_ms,
                },
                request.as_ref(),
            );
            match format {
                OutputFormat::Json => println!("{}", report_json(&run)?),
                OutputFormat::Text => println!("{}", explain_run(&run)),
            }
            Ok(if run.summary.failed == 0 { 0 } else { 1 })
        }
        Commands::Compare {
            left,
            right,
            format,
        } => {
            let left = decode_run(&fs::read_to_string(left)?)?;
            let right = decode_run(&fs::read_to_string(right)?)?;
            let report = compare_runs(&left, &right);
            match format {
                OutputFormat::Json => println!("{}", report_json(&report)?),
                OutputFormat::Text => println!("{}", explain_comparison(&report)),
            }
            Ok(if report.comparable { 0 } else { 1 })
        }
        Commands::Inspect { run, format } => {
            let run = decode_run(&fs::read_to_string(run)?)?;
            match format {
                OutputFormat::Json => println!("{}", report_json(&run)?),
                OutputFormat::Text => println!("{}", explain_run(&run)),
            }
            Ok(0)
        }
    }
}

fn command_affinity_display(cpus: &[u32]) -> String {
    cpus.iter()
        .map(u32::to_string)
        .collect::<Vec<_>>()
        .join(",")
}
