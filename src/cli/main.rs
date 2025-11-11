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
    }

    Ok(())
}
