use std::process::ExitCode;

use clap::{Parser, Subcommand};

use cli_denoiser::filters::ansi::AnsiFilter;
use cli_denoiser::filters::dedup::DedupFilter;
use cli_denoiser::filters::generic::GenericFilter;
use cli_denoiser::filters::git::GitFilter;
use cli_denoiser::filters::npm::NpmFilter;
use cli_denoiser::filters::progress::ProgressFilter;
use cli_denoiser::filters::{CommandKind, Filter};
use cli_denoiser::hooks;
use cli_denoiser::pipeline::Pipeline;
use cli_denoiser::stream;
use cli_denoiser::tracker::{FilterEvent, TrackerDb};

#[derive(Parser)]
#[command(
    name = "cli-denoiser",
    about = "Strip terminal noise for LLM agents. Zero false positives.",
    version,
    after_help = "Examples:\n  cli-denoiser git push origin main\n  cli-denoiser npm install\n  cli-denoiser install\n  cli-denoiser gain"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Run in hook mode (reads stdin, writes filtered stdout)
    #[arg(long, hide = true)]
    hook_mode: bool,

    /// Command to execute and filter
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    args: Vec<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Install hooks into all detected LLM agents
    Install,
    /// Uninstall hooks from all LLM agents
    Uninstall,
    /// Show token savings statistics
    Gain {
        /// Number of days to show stats for
        #[arg(short, long, default_value = "30")]
        days: u32,
    },
    /// Filter stdin (pipe mode)
    Filter {
        /// Command name hint for choosing the right filter
        #[arg(short, long)]
        command: Option<String>,
    },
}

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();

    // Hook mode: read stdin, filter, write stdout
    if cli.hook_mode {
        return run_hook_mode();
    }

    match cli.command {
        Some(Commands::Install) => run_install(),
        Some(Commands::Uninstall) => run_uninstall(),
        Some(Commands::Gain { days }) => run_gain(days),
        Some(Commands::Filter { command }) => run_filter_stdin(command.as_deref()),
        None => {
            if cli.args.is_empty() {
                eprintln!("Usage: cli-denoiser <command> [args...]");
                eprintln!("       cli-denoiser install");
                eprintln!("       cli-denoiser gain");
                eprintln!("Run 'cli-denoiser --help' for more info.");
                return ExitCode::from(1);
            }
            run_command(&cli.args).await
        }
    }
}

async fn run_command(args: &[String]) -> ExitCode {
    let command = &args[0];
    let cmd_args = &args[1..];
    let kind = CommandKind::detect(command);
    let pipeline = build_pipeline(&kind);

    match stream::run_filtered(command, cmd_args, &pipeline).await {
        Ok(run) => {
            if !run.stdout.output.is_empty() {
                print!("{}", run.stdout.output);
            }
            if !run.stderr.output.is_empty() {
                eprint!("{}", run.stderr.output);
            }

            if run.total_savings() > 0 {
                record_savings(command, &run);
            }

            ExitCode::from(u8::try_from(run.exit_code).unwrap_or(1))
        }
        Err(e) => {
            eprintln!("[cli-denoiser] {e}");
            ExitCode::from(1)
        }
    }
}

fn run_hook_mode() -> ExitCode {
    let mut input = String::new();
    if let Err(e) = std::io::Read::read_to_string(&mut std::io::stdin(), &mut input) {
        eprintln!("[cli-denoiser] failed to read stdin: {e}");
        return ExitCode::from(1);
    }

    let pipeline = build_pipeline(&CommandKind::Unknown);
    let result = pipeline.process(&input);

    print!("{}", result.output);

    if result.savings > 0 {
        let event = FilterEvent::new("hook", result.original_tokens, result.filtered_tokens);
        if let Ok(db) = TrackerDb::open() {
            let _ = db.record(&event);
        }
    }

    ExitCode::SUCCESS
}

fn run_filter_stdin(command_hint: Option<&str>) -> ExitCode {
    let mut input = String::new();
    if let Err(e) = std::io::Read::read_to_string(&mut std::io::stdin(), &mut input) {
        eprintln!("[cli-denoiser] failed to read stdin: {e}");
        return ExitCode::from(1);
    }

    let kind = command_hint.map_or(CommandKind::Unknown, CommandKind::detect);
    let pipeline = build_pipeline(&kind);
    let result = pipeline.process(&input);

    print!("{}", result.output);

    if result.savings > 0 {
        eprintln!(
            "[cli-denoiser] saved ~{} tokens ({:.1}%)",
            result.savings,
            result.savings_percent()
        );
    }

    ExitCode::SUCCESS
}

fn run_install() -> ExitCode {
    println!("cli-denoiser: installing hooks...\n");
    let results = hooks::install_all();
    for result in &results {
        println!("{result}");
    }
    println!("\nDone. Run 'cli-denoiser uninstall' to remove.");
    ExitCode::SUCCESS
}

fn run_uninstall() -> ExitCode {
    println!("cli-denoiser: removing hooks...\n");
    let results = hooks::uninstall_all();
    for result in &results {
        println!("{result}");
    }
    println!("\nDone.");
    ExitCode::SUCCESS
}

fn run_gain(days: u32) -> ExitCode {
    let db = match TrackerDb::open() {
        Ok(db) => db,
        Err(e) => {
            eprintln!("[cli-denoiser] failed to open tracker: {e}");
            return ExitCode::from(1);
        }
    };

    let _ = db.prune();

    let summary = match db.gain_summary(days) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("[cli-denoiser] failed to query stats: {e}");
            return ExitCode::from(1);
        }
    };

    if summary.total_events == 0 {
        println!("No filter events recorded in the last {days} days.");
        println!("Run commands through cli-denoiser to start tracking savings.");
        return ExitCode::SUCCESS;
    }

    println!("cli-denoiser savings (last {} days)\n", summary.period_days);
    println!("  Total events:    {}", summary.total_events);
    println!(
        "  Original tokens: {}",
        format_number(summary.total_original_tokens)
    );
    println!(
        "  After filter:    {}",
        format_number(summary.total_filtered_tokens)
    );
    println!(
        "  Tokens saved:    {} ({:.1}%)",
        format_number(summary.total_savings),
        summary.savings_percent
    );

    if !summary.top_commands.is_empty() {
        println!("\n  Top commands by savings:");
        for cmd in &summary.top_commands {
            println!(
                "    {:<20} {:>6} saved  ({} events)",
                cmd.command,
                format_number(cmd.savings),
                cmd.events
            );
        }
    }

    ExitCode::SUCCESS
}

fn build_pipeline(kind: &CommandKind) -> Pipeline {
    let mut pipeline = Pipeline::new();

    // Universal filters (always applied)
    pipeline.add_filter(Box::new(AnsiFilter));
    pipeline.add_filter(Box::new(ProgressFilter));
    pipeline.add_filter(Box::new(DedupFilter::new()));

    // Command-specific filters
    let specific: Box<dyn Filter> = match kind {
        CommandKind::Git => Box::new(GitFilter),
        CommandKind::Npm => Box::new(NpmFilter),
        _ => Box::new(GenericFilter),
    };
    pipeline.add_filter(specific);

    pipeline
}

fn record_savings(command: &str, run: &stream::FilteredRun) {
    let event = FilterEvent::new(
        command,
        run.total_original_tokens(),
        run.stdout.filtered_tokens + run.stderr.filtered_tokens,
    );
    if let Ok(db) = TrackerDb::open() {
        let _ = db.record(&event);
    }
}

#[allow(clippy::cast_precision_loss)]
fn format_number(n: usize) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}K", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}
