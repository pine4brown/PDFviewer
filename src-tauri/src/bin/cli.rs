//! `wafflematrix-cli` — headless PDF diff comparison and accuracy evaluation.
//!
//! Subcommands:
//! * `compare`  — compare two PDFs and print / export a report.
//! * `eval`     — run the ground-truth corpus and report accuracy metrics.
//! * `gen`      — regenerate the synthetic corpus (PDFs + ground truth).
//! * `golden`   — freeze the current text-mode output of a real PDF pair.
//! * `list`     — list the cases in a corpus directory.
//!
//! The `eval` command exits non-zero when the primary metric (text-content F1)
//! drops below `--min-f1`, so CI can gate on diff-detection accuracy.

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

use wafflematrix_lib::bench::case::GroundTruth;
use wafflematrix_lib::bench::eval::{self, EvalConfig};
use wafflematrix_lib::diff::report::DiffMode;

#[derive(Parser)]
#[command(
    name = "wafflematrix-cli",
    version,
    about = "PDF diff comparison and accuracy evaluation tool",
    arg_required_else_help = true
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Compare two PDFs and print / write a report.
    Compare(CompareArgs),
    /// Run the ground-truth corpus and report accuracy metrics.
    Eval(EvalArgs),
    /// Regenerate the synthetic corpus.
    Gen(GenArgs),
    /// Freeze the current text-mode output of a real PDF pair as a golden case.
    Golden(GoldenArgs),
    /// List the cases in a corpus directory.
    List(ListArgs),
}

#[derive(clap::Args)]
struct CompareArgs {
    /// Path to the old (baseline) PDF.
    #[arg(long)]
    old: String,
    /// Path to the new (revised) PDF.
    #[arg(long)]
    new: String,
    /// Comparison mode: text | visual | hybrid.
    #[arg(long, default_value = "text")]
    mode: String,
    /// Write the report to this path (xlsx / csv / json / html by extension).
    /// Use "-" to write JSON to stdout.
    #[arg(long)]
    output: Option<String>,
    /// Print the JSON report to stdout.
    #[arg(long)]
    json: bool,
    /// PDFium shared library path (default: auto-detect).
    #[arg(long)]
    pdfium: Option<String>,
}

#[derive(clap::Args)]
struct EvalArgs {
    /// Corpus directory containing the test cases.
    #[arg(long, default_value_t = default_corpus())]
    corpus: String,
    /// Only evaluate these cases (repeatable).
    #[arg(long)]
    case: Vec<String>,
    /// Modes to evaluate, comma-separated: text,visual,hybrid (default all).
    #[arg(long)]
    modes: Option<String>,
    /// Minimum text-content F1; the command fails when any mode is below it.
    #[arg(long, default_value_t = 0.0)]
    min_f1: f64,
    /// Minimum visual region F1 (visual mode); the command fails when below.
    #[arg(long)]
    min_visual_f1: Option<f64>,
    /// Overlap threshold for rect matching (0..1, default 0.5).
    #[arg(long, default_value_t = 0.5)]
    threshold: f64,
    /// Write the full evaluation summary (JSON) to this file.
    #[arg(long)]
    report: Option<String>,
    /// Emit a TSV per-case table (for CI dashboards) instead of markdown.
    #[arg(long)]
    ci: bool,
    /// PDFium shared library path (default: auto-detect).
    #[arg(long)]
    pdfium: Option<String>,
}

#[derive(clap::Args)]
struct GenArgs {
    /// Corpus directory to (re)generate.
    #[arg(long, default_value_t = default_corpus())]
    corpus: String,
    /// Seed for reproducible generation.
    #[arg(long, default_value_t = 42)]
    seed: u64,
    /// Overwrite existing cases.
    #[arg(long)]
    force: bool,
    /// PDFium shared library path (default: auto-detect).
    #[arg(long)]
    pdfium: Option<String>,
}

#[derive(clap::Args)]
struct GoldenArgs {
    /// Corpus directory the case is written to.
    #[arg(long, default_value_t = default_corpus())]
    corpus: String,
    /// Case name (directory + id).
    #[arg(long)]
    name: String,
    /// Short description.
    #[arg(long)]
    description: Option<String>,
    /// Old PDF.
    #[arg(long)]
    old: String,
    /// New PDF.
    #[arg(long)]
    new: String,
    /// PDFium shared library path (default: auto-detect).
    #[arg(long)]
    pdfium: Option<String>,
}

#[derive(clap::Args)]
struct ListArgs {
    /// Corpus directory to list.
    #[arg(long, default_value_t = default_corpus())]
    corpus: String,
}

fn default_corpus() -> String {
    std::env::var("CARGO_MANIFEST_DIR")
        .map(|m| PathBuf::from(m).join("../testdata/corpus").to_string_lossy().into_owned())
        .unwrap_or_else(|_| "testdata/corpus".to_string())
}

fn resolve_lib(flag: Option<String>) -> Result<String, String> {
    if let Some(p) = flag {
        return Ok(p);
    }
    wafflematrix_lib::pdf::engine::resolve_pdfium_lib_path()
}

fn parse_modes(s: &str) -> Result<Vec<DiffMode>, String> {
    s.split(',')
        .map(|m| m.trim())
        .filter(|m| !m.is_empty())
        .map(|m| match m.to_ascii_lowercase().as_str() {
            "text" => Ok(DiffMode::Text),
            "visual" => Ok(DiffMode::Visual),
            "hybrid" => Ok(DiffMode::Hybrid),
            other => Err(format!("unknown mode: '{other}'")),
        })
        .collect()
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(1)
        }
    }
}

fn run() -> Result<ExitCode, String> {
    let cli = Cli::parse();
    match cli.command {
        Command::Compare(a) => cmd_compare(a),
        Command::Eval(a) => cmd_eval(a),
        Command::Gen(a) => cmd_gen(a),
        Command::Golden(a) => cmd_golden(a),
        Command::List(a) => cmd_list(a),
    }
}

// ---- compare ----------------------------------------------------------------

fn cmd_compare(a: CompareArgs) -> Result<ExitCode, String> {
    let lib = resolve_lib(a.pdfium)?;
    let mode = DiffMode::from_str(&a.mode);

    let report = wafflematrix_lib::diff::loader::compare_pdf_files(&lib, &a.old, &a.new, mode)?;

    let total = report.total_changes();
    eprintln!(
        "[compare] old={} new={} mode={} pages={} changes={total}",
        a.old, a.new, mode.as_str(),
        report.pages.len()
    );

    if let Some(out) = &a.output {
        if out == "-" {
            print_report_json(&report);
        } else {
            let fmt = extension_of(out).unwrap_or_else(|| "xlsx".to_string());
            export_report(&report, out, &fmt)?;
        }
    }
    if a.json {
        print_report_json(&report);
    }

    Ok(ExitCode::SUCCESS)
}

fn print_report_json(report: &wafflematrix_lib::diff::report::DiffReport) {
    let json = serde_json::to_string_pretty(report).expect("report serialises");
    println!("{json}");
}

fn export_report(
    report: &wafflematrix_lib::diff::report::DiffReport,
    path: &str,
    format: &str,
) -> Result<(), String> {
    match format {
        "xlsx" => wafflematrix_lib::diff::export::export_xlsx(report, path),
        "csv" => wafflematrix_lib::diff::export::export_csv(report, path),
        "json" => wafflematrix_lib::diff::export::export_json(report, path),
        "html" => wafflematrix_lib::diff::export::export_html(report, path),
        other => Err(format!("unsupported export format: '{other}'")),
    }
}

fn extension_of(path: &str) -> Option<String> {
    std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
}

// ---- eval -------------------------------------------------------------------

fn cmd_eval(a: EvalArgs) -> Result<ExitCode, String> {
    let lib = resolve_lib(a.pdfium)?;

    if !(0.0..=1.0).contains(&a.min_f1) {
        return Err("--min-f1 must be in 0..=1".into());
    }
    if let Some(v) = a.min_visual_f1 {
        if !(0.0..=1.0).contains(&v) {
            return Err("--min-visual-f1 must be in 0..=1".into());
        }
    }
    if !(0.0..=1.0).contains(&a.threshold) {
        return Err("--threshold must be in 0..=1".into());
    }

    let modes = match &a.modes {
        Some(s) => parse_modes(s)?,
        None => Vec::new(),
    };

    let cfg = EvalConfig {
        lib_path: lib,
        corpus_dir: std::path::PathBuf::from(a.corpus),
        modes,
        cases: a.case,
        overlap_threshold: a.threshold,
    };

    let summary = eval::run_eval(&cfg)?;

    if a.ci {
        print!("{}", eval::format_tsv(&summary));
    } else {
        print!("{}", eval::format_markdown(&summary));
    }

    if let Some(path) = &a.report {
        let json = serde_json::to_string_pretty(&summary).expect("summary serialises");
        std::fs::write(path, json).map_err(|e| format!("write {}: {e}", path))?;
    }

    // Gate on the primary metric: text-content F1 for every mode that defines it.
    let mut lowest: Option<f64> = None;
    for m in &summary.by_mode {
        if let Some(t) = &m.text_content {
            lowest = Some(lowest.map_or(t.f1, |x: f64| x.min(t.f1)));
        }
    }

    match lowest {
        Some(f1) if f1 < a.min_f1 => {
            eprintln!(
                "eval failed: text-content F1 {f1:.4} is below --min-f1 {:.4}",
                a.min_f1
            );
            return Ok(ExitCode::from(1));
        }
        _ => {}
    }

    // Gate on visual region F1 for the visual mode.
    if let Some(min) = a.min_visual_f1 {
        for m in &summary.by_mode {
            if m.mode == "visual" {
                if let Some(r) = &m.region {
                    if r.f1 < min {
                        eprintln!(
                            "eval failed: visual region F1 {:.4} is below --min-visual-f1 {min:.4}",
                            r.f1
                        );
                        return Ok(ExitCode::from(1));
                    }
                }
            }
        }
    }

    Ok(ExitCode::SUCCESS)
}

// ---- gen --------------------------------------------------------------------

fn cmd_gen(a: GenArgs) -> Result<ExitCode, String> {
    let lib = resolve_lib(a.pdfium)?;
    let outcomes = wafflematrix_lib::bench::gen::generate_corpus(&lib, &std::path::PathBuf::from(a.corpus.clone()), a.seed, a.force)?;

    for o in &outcomes {
        let state = if o.skipped { "skip" } else { "ok" };
        println!("[{state}] {} — {}", o.name, o.description);
    }
    let generated = outcomes.iter().filter(|o| !o.skipped).count();
    eprintln!("[gen] {} case(s) written to {}", generated, std::path::PathBuf::from(&a.corpus).display());
    Ok(ExitCode::SUCCESS)
}

// ---- golden ---------------------------------------------------------------

fn cmd_golden(a: GoldenArgs) -> Result<ExitCode, String> {
    let lib = resolve_lib(a.pdfium)?;
    let description = a
        .description
        .unwrap_or_else(|| "Golden snapshot of real PDF output (text mode).".to_string());

    let gt: GroundTruth = wafflematrix_lib::bench::gen::write_golden_case(
        &lib,
        &std::path::PathBuf::from(a.corpus),
        &a.name,
        &description,
        &a.old,
        &a.new,
    )?;

    println!(
        "golden case '{}' written with {} page(s) of ground truth",
        a.name,
        gt.pages.len()
    );
    Ok(ExitCode::SUCCESS)
}

// ---- list ------------------------------------------------------------------

fn cmd_list(a: ListArgs) -> Result<ExitCode, String> {
    let cases = eval::discover_cases(std::path::Path::new(&a.corpus))?;
    if cases.is_empty() {
        eprintln!("[list] no cases found in {}", std::path::PathBuf::from(&a.corpus).display());
        return Ok(ExitCode::from(1));
    }
    for c in &cases {
        println!("{}\t{}", c.name, c.description);
    }
    eprintln!("[list] {} case(s)", cases.len());
    Ok(ExitCode::SUCCESS)
}
