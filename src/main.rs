use std::process::ExitCode;

use clap::{Parser, Subcommand};

use cli_denoiser::filters::CommandKind;
use cli_denoiser::tracker::{FilterEvent, TrackerDb};
use cli_denoiser::{bench, build_pipeline, hooks, stream};

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

    /// Show debug output (which filters matched, what was dropped)
    #[arg(long, global = true)]
    debug: bool,

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
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Filter stdin (pipe mode)
    Filter {
        /// Command name hint for choosing the right filter
        #[arg(short, long)]
        command: Option<String>,
    },
    /// Run benchmark suite and output results
    Bench {
        /// Output JSON results to file
        #[arg(short, long)]
        output: Option<String>,
    },
    /// Daily savings report with trend data
    Report {
        /// Number of days to show
        #[arg(short, long, default_value = "7")]
        days: u32,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Show recent filter event log
    Log {
        /// Number of recent events to show
        #[arg(short = 'n', long, default_value = "20")]
        limit: u32,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();

    // Hook mode: read stdin, filter, write stdout
    if cli.hook_mode {
        return run_hook_mode();
    }

    let debug = cli.debug;

    match cli.command {
        Some(Commands::Install) => run_install(),
        Some(Commands::Uninstall) => run_uninstall(),
        Some(Commands::Gain { days, json }) => run_gain(days, json),
        Some(Commands::Filter { command }) => run_filter_stdin(command.as_deref(), debug),
        Some(Commands::Bench { output }) => run_bench(output.as_deref()),
        Some(Commands::Report { days, json }) => run_report(days, json),
        Some(Commands::Log { limit, json }) => run_log(limit, json),
        None => {
            if cli.args.is_empty() {
                eprintln!("Usage: cli-denoiser <command> [args...]");
                eprintln!("       cli-denoiser install");
                eprintln!("       cli-denoiser gain");
                eprintln!("       cli-denoiser report");
                eprintln!("       cli-denoiser log");
                eprintln!("Run 'cli-denoiser --help' for more info.");
                return ExitCode::from(1);
            }
            run_command(&cli.args, debug).await
        }
    }
}

async fn run_command(args: &[String], debug: bool) -> ExitCode {
    let command = &args[0];
    let cmd_args = &args[1..];
    let kind = CommandKind::detect(command);
    let pipeline = build_pipeline(&kind, debug);

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

    let pipeline = build_pipeline(&CommandKind::Unknown, false);
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

fn run_filter_stdin(command_hint: Option<&str>, debug: bool) -> ExitCode {
    let mut input = String::new();
    if let Err(e) = std::io::Read::read_to_string(&mut std::io::stdin(), &mut input) {
        eprintln!("[cli-denoiser] failed to read stdin: {e}");
        return ExitCode::from(1);
    }

    let kind = command_hint.map_or(CommandKind::Unknown, CommandKind::detect);
    let pipeline = build_pipeline(&kind, debug);
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

fn run_gain(days: u32, json: bool) -> ExitCode {
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

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&summary).unwrap_or_default()
        ); // allow-unwrap: serialization of known types
        return ExitCode::SUCCESS;
    }

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

fn run_bench(output_path: Option<&str>) -> ExitCode {
    let results = bench::run_all();
    let json = match serde_json::to_string_pretty(&results) {
        Ok(j) => j,
        Err(e) => {
            eprintln!("[cli-denoiser] failed to serialize results: {e}");
            return ExitCode::from(1);
        }
    };

    // Print summary to terminal
    bench::print_summary(&results);

    // Write JSON if output path specified
    if let Some(path) = output_path {
        if let Err(e) = std::fs::write(path, &json) {
            eprintln!("[cli-denoiser] failed to write {path}: {e}");
            return ExitCode::from(1);
        }
        println!("\nJSON results written to: {path}");
    }

    ExitCode::SUCCESS
}

fn run_report(days: u32, json: bool) -> ExitCode {
    let db = match TrackerDb::open() {
        Ok(db) => db,
        Err(e) => {
            eprintln!("[cli-denoiser] failed to open tracker: {e}");
            return ExitCode::from(1);
        }
    };

    let daily = match db.daily_report(days) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("[cli-denoiser] failed to query report: {e}");
            return ExitCode::from(1);
        }
    };

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&daily).unwrap_or_default()
        ); // allow-unwrap: serialization of known types
        return ExitCode::SUCCESS;
    }

    if daily.is_empty() {
        println!("No filter events in the last {days} days.");
        println!("Run commands through cli-denoiser to start tracking.");
        return ExitCode::SUCCESS;
    }

    println!("cli-denoiser daily report (last {days} days)\n");
    println!(
        "  {:<12} {:>6} {:>10} {:>10} {:>10} {:>8}",
        "Date", "Events", "Original", "Filtered", "Saved", "Savings"
    );
    println!("  {}", "-".repeat(62));

    for day in &daily {
        println!(
            "  {:<12} {:>6} {:>10} {:>10} {:>10} {:>7.1}%",
            day.date,
            day.events,
            format_number(day.original_tokens),
            format_number(day.filtered_tokens),
            format_number(day.savings),
            day.savings_percent
        );
    }

    let total_events: usize = daily.iter().map(|d| d.events).sum();
    let total_savings: usize = daily.iter().map(|d| d.savings).sum();
    println!("  {}", "-".repeat(62));
    println!(
        "  {:<12} {:>6} {:>10} tokens saved total",
        "TOTAL",
        total_events,
        format_number(total_savings)
    );

    ExitCode::SUCCESS
}

fn run_log(limit: u32, json: bool) -> ExitCode {
    let db = match TrackerDb::open() {
        Ok(db) => db,
        Err(e) => {
            eprintln!("[cli-denoiser] failed to open tracker: {e}");
            return ExitCode::from(1);
        }
    };

    let events = match db.recent_events(limit) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("[cli-denoiser] failed to query events: {e}");
            return ExitCode::from(1);
        }
    };

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&events).unwrap_or_default()
        ); // allow-unwrap: serialization of known types
        return ExitCode::SUCCESS;
    }

    if events.is_empty() {
        println!("No filter events recorded yet.");
        return ExitCode::SUCCESS;
    }

    println!(
        "  {:<20} {:>10} {:>10} {:>8} {:>8}",
        "Timestamp", "Command", "Original", "Saved", "Savings"
    );
    println!("  {}", "-".repeat(62));

    for event in &events {
        let ts = event
            .timestamp
            .get(..19)
            .unwrap_or(&event.timestamp)
            .replace('T', " ");
        #[allow(clippy::cast_precision_loss)]
        let pct = if event.original_tokens == 0 {
            0.0
        } else {
            (event.savings as f64 / event.original_tokens as f64) * 100.0
        };
        println!(
            "  {:<20} {:>10} {:>10} {:>8} {:>7.1}%",
            ts, event.command, event.original_tokens, event.savings, pct
        );
    }

    ExitCode::SUCCESS
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
