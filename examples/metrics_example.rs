/// Example demonstrating Prometheus metrics integration
///
/// This example shows how to:
/// - Initialize metrics
/// - Track HTTP requests
/// - Track LLM operations
/// - Track incident processing
/// - Export metrics
///
/// Run with: cargo run --example metrics_example

use llm_incident_manager::metrics::{
    decorators::{IncidentTracker, LLMCallTracker, PlaybookTracker},
    init_metrics, gather_metrics,
    HTTP_REQUESTS_TOTAL, INCIDENTS_TOTAL, LLM_REQUESTS_TOTAL,
};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    println!("=== Prometheus Metrics Example ===\n");

    // Initialize metrics
    println!("1. Initializing metrics...");
    init_metrics()?;
    println!("   ✓ Metrics initialized\n");

    // Simulate HTTP requests
    println!("2. Simulating HTTP requests...");
    for i in 0..5 {
        HTTP_REQUESTS_TOTAL
            .with_label_values(&["GET", "/api/incidents", "200"])
            .inc();
        println!("   Request {}/5 tracked", i + 1);
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    println!("   ✓ HTTP requests tracked\n");

    // Simulate LLM operations
    println!("3. Simulating LLM operations...");

    // Example 1: Successful LLM call
    let tracker = LLMCallTracker::start("openai", "gpt-4", "incident_analysis");
    tokio::time::sleep(Duration::from_millis(500)).await;
    tracker.success(150, 75, 0.0025); // 150 input tokens, 75 output, $0.0025
    println!("   ✓ Successful LLM call tracked");

    // Example 2: Failed LLM call
    let tracker = LLMCallTracker::start("anthropic", "claude-3-opus", "enrichment");
    tokio::time::sleep(Duration::from_millis(200)).await;
    tracker.error("rate_limit");
    println!("   ✓ Failed LLM call tracked\n");

    // Simulate incident processing
    println!("4. Simulating incident processing...");

    // Critical incident
    let tracker = IncidentTracker::start("critical");
    tokio::time::sleep(Duration::from_millis(300)).await;
    tracker.success("resolved");
    println!("   ✓ Critical incident processed");

    // Warning incident
    let tracker = IncidentTracker::start("warning");
    tokio::time::sleep(Duration::from_millis(150)).await;
    tracker.success("acknowledged");
    println!("   ✓ Warning incident processed\n");

    // Simulate playbook execution
    println!("5. Simulating playbook execution...");
    let tracker = PlaybookTracker::start("auto-remediation");
    tokio::time::sleep(Duration::from_millis(1000)).await;
    tracker.success();
    println!("   ✓ Playbook executed\n");

    // Export metrics
    println!("6. Exporting metrics...\n");
    let metrics_output = gather_metrics();

    // Print summary
    println!("=== Metrics Summary ===");
    let lines: Vec<&str> = metrics_output.lines().collect();
    let metric_count = lines.iter().filter(|l| l.starts_with("# TYPE")).count();
    println!("Total metrics defined: {}", metric_count);

    // Show some key metrics
    println!("\n=== Sample Metrics ===");
    for line in lines.iter() {
        if line.contains("http_requests_total") && line.contains("GET") {
            println!("{}", line);
        } else if line.contains("llm_requests_total") && !line.starts_with("#") {
            println!("{}", line);
        } else if line.contains("incidents_total") && !line.starts_with("#") {
            println!("{}", line);
        }
    }

    println!("\n=== Full Metrics Output ===");
    println!("(First 50 lines)\n");
    for (i, line) in lines.iter().take(50).enumerate() {
        println!("{:3}: {}", i + 1, line);
    }

    if lines.len() > 50 {
        println!("\n... ({} more lines)", lines.len() - 50);
    }

    println!("\n✓ Example completed successfully!");
    println!("\nTo view all metrics, run:");
    println!("  cargo run --example metrics_example 2>/dev/null | grep -A 1000 'Full Metrics'");

    Ok(())
}
