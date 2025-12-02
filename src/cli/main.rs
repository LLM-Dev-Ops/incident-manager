use clap::{Parser, Subcommand};
use reqwest::Client;
use serde_json::json;
use std::error::Error;

#[derive(Parser)]
#[command(name = "llm-im-cli")]
#[command(about = "LLM Incident Manager CLI", long_about = None)]
struct Cli {
    #[arg(short, long, default_value = "http://localhost:8080")]
    endpoint: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Submit an alert
    Alert {
        #[arg(short, long)]
        source: String,

        #[arg(short, long)]
        title: String,

        #[arg(short, long)]
        description: String,

        #[arg(short = 'S', long, default_value = "P2")]
        severity: String,

        #[arg(short = 'T', long, default_value = "Application")]
        alert_type: String,
    },

    /// List incidents
    List {
        #[arg(short, long, default_value = "0")]
        page: u32,

        #[arg(short = 's', long, default_value = "20")]
        page_size: u32,

        #[arg(short = 'a', long)]
        active_only: bool,
    },

    /// Get incident details
    Get {
        #[arg(value_name = "INCIDENT_ID")]
        id: String,
    },

    /// Resolve an incident
    Resolve {
        #[arg(value_name = "INCIDENT_ID")]
        id: String,

        #[arg(short, long)]
        resolved_by: String,

        #[arg(short, long)]
        notes: String,

        #[arg(short = 'c', long)]
        root_cause: Option<String>,
    },

    /// Check server health
    Health,

    /// Run all benchmarks and output results
    #[command(name = "run")]
    Run {
        /// Output directory for benchmark results
        #[arg(short, long, default_value = "benchmarks/output")]
        output: String,

        /// Only run specific target (by ID)
        #[arg(short, long)]
        target: Option<String>,

        /// Output format: json, markdown, or both
        #[arg(short, long, default_value = "both")]
        format: String,

        /// Quiet mode - only output results, no progress
        #[arg(short, long)]
        quiet: bool,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();
    let client = Client::new();

    match cli.command {
        Commands::Alert {
            source,
            title,
            description,
            severity,
            alert_type,
        } => {
            let response = client
                .post(format!("{}/v1/alerts", cli.endpoint))
                .json(&json!({
                    "source": source,
                    "title": title,
                    "description": description,
                    "severity": severity,
                    "alert_type": alert_type,
                }))
                .send()
                .await?;

            let body: serde_json::Value = response.json().await?;
            println!("{}", serde_json::to_string_pretty(&body)?);
        }

        Commands::List {
            page,
            page_size,
            active_only,
        } => {
            let mut url = format!("{}/v1/incidents?page={}&page_size={}", cli.endpoint, page, page_size);
            if active_only {
                url.push_str("&active_only=true");
            }

            let response = client.get(&url).send().await?;
            let body: serde_json::Value = response.json().await?;
            println!("{}", serde_json::to_string_pretty(&body)?);
        }

        Commands::Get { id } => {
            let response = client
                .get(format!("{}/v1/incidents/{}", cli.endpoint, id))
                .send()
                .await?;

            let body: serde_json::Value = response.json().await?;
            println!("{}", serde_json::to_string_pretty(&body)?);
        }

        Commands::Resolve {
            id,
            resolved_by,
            notes,
            root_cause,
        } => {
            let response = client
                .post(format!("{}/v1/incidents/{}/resolve", cli.endpoint, id))
                .json(&json!({
                    "resolved_by": resolved_by,
                    "method": "Manual",
                    "notes": notes,
                    "root_cause": root_cause,
                }))
                .send()
                .await?;

            let body: serde_json::Value = response.json().await?;
            println!("{}", serde_json::to_string_pretty(&body)?);
        }

        Commands::Health => {
            let response = client
                .get(format!("{}/health", cli.endpoint))
                .send()
                .await?;

            let body: serde_json::Value = response.json().await?;
            println!("{}", serde_json::to_string_pretty(&body)?);
        }

        Commands::Run {
            output,
            target,
            format,
            quiet,
        } => {
            use llm_incident_manager::adapters::all_targets;
            use llm_incident_manager::benchmarks::{io, markdown, BenchmarkResult};
            use std::fs;
            use std::time::Instant;

            // Create output directories
            let raw_dir = format!("{}/raw", output);
            fs::create_dir_all(&raw_dir)?;

            if !quiet {
                println!("LLM Incident Manager Benchmark Runner");
                println!("=====================================");
                println!();
            }

            let targets = all_targets();
            let mut results: Vec<BenchmarkResult> = Vec::new();

            // Filter targets if specific one requested
            let targets_to_run: Vec<_> = if let Some(ref target_id) = target {
                targets
                    .into_iter()
                    .filter(|t| t.id() == *target_id)
                    .collect()
            } else {
                targets
            };

            if targets_to_run.is_empty() {
                if let Some(ref target_id) = target {
                    eprintln!("Error: No benchmark target found with ID '{}'", target_id);
                    eprintln!("Available targets:");
                    for t in all_targets() {
                        eprintln!("  - {}", t.id());
                    }
                    std::process::exit(1);
                }
            }

            if !quiet {
                println!("Running {} benchmark targets...", targets_to_run.len());
                println!();
            }

            let total_start = Instant::now();

            for t in targets_to_run {
                let id = t.id();
                if !quiet {
                    print!("  {} ... ", id);
                }

                let start = Instant::now();
                let result = t.run().await;
                let elapsed = start.elapsed();

                if !quiet {
                    if result.is_success() {
                        println!("OK ({:.2}ms)", elapsed.as_secs_f64() * 1000.0);
                    } else {
                        println!("FAILED ({:.2}ms)", elapsed.as_secs_f64() * 1000.0);
                    }
                }

                results.push(result);
            }

            let total_elapsed = total_start.elapsed();

            if !quiet {
                println!();
                println!("Completed {} benchmarks in {:.2}s", results.len(), total_elapsed.as_secs_f64());
                let successful = results.iter().filter(|r| r.is_success()).count();
                println!("  Successful: {}", successful);
                println!("  Failed: {}", results.len() - successful);
            }

            // Write results based on format
            match format.as_str() {
                "json" => {
                    io::write_results(&results)?;
                    io::write_combined_results(&results)?;
                    if !quiet {
                        println!();
                        println!("Results written to: {}/raw/", output);
                    }
                }
                "markdown" => {
                    let summary_path = format!("{}/summary.md", output);
                    markdown::write_summary_to_path(&results, &summary_path)?;
                    if !quiet {
                        println!();
                        println!("Summary written to: {}", summary_path);
                    }
                }
                "both" | _ => {
                    io::write_results(&results)?;
                    io::write_combined_results(&results)?;
                    let summary_path = format!("{}/summary.md", output);
                    markdown::write_summary_to_path(&results, &summary_path)?;
                    if !quiet {
                        println!();
                        println!("Results written to:");
                        println!("  - {}/raw/ (JSON)", output);
                        println!("  - {} (Markdown)", summary_path);
                    }
                }
            }

            // Output JSON to stdout if quiet mode
            if quiet {
                println!("{}", serde_json::to_string_pretty(&results)?);
            }
        }
    }

    Ok(())
}
