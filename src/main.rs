mod crawler;
mod extractor;
mod lead;
mod output;

use std::path::Path;

use clap::Parser;
use colored::Colorize;

/// Extract contact information (emails, phones, social links) from public web pages.
#[derive(Parser, Debug)]
#[command(name = "leadextract", version, about, long_about = None)]
struct Cli {
    /// URL to extract from, or path to a text file containing URLs (one per line).
    target: String,

    /// Maximum crawl depth (0 = single page only).
    #[arg(short, long, default_value_t = 0)]
    depth: u32,

    /// Output file path (.json or .csv).
    #[arg(short, long)]
    output: Option<String>,

    /// Rate limit in milliseconds between requests to the same domain.
    #[arg(long, default_value_t = 1000)]
    rate_limit: u64,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let urls = resolve_targets(&cli.target);
    if urls.is_empty() {
        eprintln!("{}", "No valid URLs provided.".red());
        std::process::exit(1);
    }

    eprintln!(
        "{}",
        format!(
            "leadextract v{} - processing {} URL(s), depth={}",
            env!("CARGO_PKG_VERSION"),
            urls.len(),
            cli.depth
        )
        .cyan()
        .bold()
    );

    let mut all_leads = Vec::new();

    for url in &urls {
        eprintln!();
        eprintln!("{}", format!("Crawling: {url}").cyan());

        match crawler::crawl(url, cli.depth, cli.rate_limit).await {
            Ok(leads) => all_leads.extend(leads),
            Err(e) => {
                eprintln!("{}", format!("Error crawling {url}: {e}").red());
            }
        }
    }

    // Print results to terminal
    output::print_results(&all_leads);

    // Export if requested
    if let Some(ref output_path) = cli.output {
        let path = Path::new(output_path);
        let result = match path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|s| s.to_lowercase())
            .as_deref()
        {
            Some("json") => output::export_json(&all_leads, path),
            Some("csv") => output::export_csv(&all_leads, path),
            _ => {
                eprintln!(
                    "{}",
                    "Unknown output format. Use .json or .csv extension.".red()
                );
                std::process::exit(1);
            }
        };

        if let Err(e) = result {
            eprintln!("{}", format!("Export error: {e}").red());
            std::process::exit(1);
        }
    }
}

/// Resolve the target argument into a list of URLs.
/// If it looks like a file path and the file exists, read URLs from it.
/// Otherwise treat it as a single URL.
fn resolve_targets(target: &str) -> Vec<String> {
    // If it looks like a URL, use it directly
    if target.starts_with("http://") || target.starts_with("https://") {
        return vec![target.to_string()];
    }

    // Try to read as a file
    let path = Path::new(target);
    if path.exists() {
        match std::fs::read_to_string(path) {
            Ok(contents) => {
                return contents
                    .lines()
                    .map(|l| l.trim().to_string())
                    .filter(|l| !l.is_empty() && !l.starts_with('#'))
                    .filter(|l| l.starts_with("http://") || l.starts_with("https://"))
                    .collect();
            }
            Err(e) => {
                eprintln!("Failed to read file {target}: {e}");
                return Vec::new();
            }
        }
    }

    // Maybe they forgot the scheme
    if target.contains('.') && !target.contains(' ') {
        eprintln!(
            "{}",
            format!("Assuming https:// for: {target}").yellow()
        );
        return vec![format!("https://{target}")];
    }

    eprintln!("Not a valid URL or file: {target}");
    Vec::new()
}
