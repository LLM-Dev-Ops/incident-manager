//! Report export formats and utilities

use crate::analytics::error::{AnalyticsError, AnalyticsResult};
use crate::analytics::reports::Report;
use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio::fs;
use tokio::io::AsyncWriteExt;

/// Export format for reports
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ExportFormat {
    Json,
    Csv,
    Pdf,
    Html,
}

impl ExportFormat {
    /// Get file extension for this format
    pub fn extension(&self) -> &str {
        match self {
            ExportFormat::Json => "json",
            ExportFormat::Csv => "csv",
            ExportFormat::Pdf => "pdf",
            ExportFormat::Html => "html",
        }
    }

    /// Get MIME type for this format
    pub fn mime_type(&self) -> &str {
        match self {
            ExportFormat::Json => "application/json",
            ExportFormat::Csv => "text/csv",
            ExportFormat::Pdf => "application/pdf",
            ExportFormat::Html => "text/html",
        }
    }
}

/// Report exporter
pub struct ReportExporter;

impl ReportExporter {
    /// Export a report to a specific format
    pub async fn export(
        report: &Report,
        format: ExportFormat,
        output_path: &Path,
    ) -> AnalyticsResult<Vec<u8>> {
        match format {
            ExportFormat::Json => Self::export_json(report, output_path).await,
            ExportFormat::Csv => Self::export_csv(report, output_path).await,
            ExportFormat::Pdf => Self::export_pdf(report, output_path).await,
            ExportFormat::Html => Self::export_html(report, output_path).await,
        }
    }

    /// Export report as JSON
    async fn export_json(report: &Report, output_path: &Path) -> AnalyticsResult<Vec<u8>> {
        let json = serde_json::to_string_pretty(report)
            .map_err(|e| AnalyticsError::ExportFailed(format!("JSON serialization failed: {}", e)))?;

        let bytes = json.as_bytes().to_vec();

        // Write to file
        fs::write(output_path, &bytes)
            .await
            .map_err(|e| AnalyticsError::ExportFailed(format!("Failed to write file: {}", e)))?;

        Ok(bytes)
    }

    /// Export report as CSV
    async fn export_csv(report: &Report, output_path: &Path) -> AnalyticsResult<Vec<u8>> {
        // Convert the report data to CSV format
        let mut csv_content = String::new();

        // Header
        csv_content.push_str("Report Title,Report Type,Generated At,Period Start,Period End,Summary\n");

        // Report metadata
        csv_content.push_str(&format!(
            "\"{}\",\"{:?}\",\"{}\",\"{}\",\"{}\",\"{}\"\n",
            Self::escape_csv(&report.title),
            report.report_type,
            report.generated_at.to_rfc3339(),
            report.period_start.to_rfc3339(),
            report.period_end.to_rfc3339(),
            Self::escape_csv(&report.summary)
        ));

        // Add a blank line
        csv_content.push('\n');

        // Extract data fields if possible
        if let Some(obj) = report.data.as_object() {
            csv_content.push_str("Field,Value\n");
            for (key, value) in obj {
                let value_str = match value {
                    serde_json::Value::String(s) => s.clone(),
                    serde_json::Value::Number(n) => n.to_string(),
                    serde_json::Value::Bool(b) => b.to_string(),
                    serde_json::Value::Null => "null".to_string(),
                    _ => serde_json::to_string(value).unwrap_or_default(),
                };
                csv_content.push_str(&format!("\"{}\",\"{}\"\n", Self::escape_csv(key), Self::escape_csv(&value_str)));
            }
        }

        let bytes = csv_content.as_bytes().to_vec();

        // Write to file
        fs::write(output_path, &bytes)
            .await
            .map_err(|e| AnalyticsError::ExportFailed(format!("Failed to write CSV file: {}", e)))?;

        Ok(bytes)
    }

    /// Export report as PDF (placeholder - would need a PDF library)
    async fn export_pdf(report: &Report, output_path: &Path) -> AnalyticsResult<Vec<u8>> {
        // For now, we'll create a simple text-based PDF placeholder
        // In a production system, you'd use a library like printpdf or wkhtmltopdf

        let pdf_content = format!(
            "PDF Export - Report: {}\n\n\
             Type: {:?}\n\
             Generated: {}\n\
             Period: {} to {}\n\n\
             Summary:\n{}\n\n\
             Data:\n{}\n",
            report.title,
            report.report_type,
            report.generated_at.to_rfc3339(),
            report.period_start.to_rfc3339(),
            report.period_end.to_rfc3339(),
            report.summary,
            serde_json::to_string_pretty(&report.data).unwrap_or_default()
        );

        let bytes = pdf_content.as_bytes().to_vec();

        fs::write(output_path, &bytes)
            .await
            .map_err(|e| AnalyticsError::ExportFailed(format!("Failed to write PDF file: {}", e)))?;

        Ok(bytes)
    }

    /// Export report as HTML
    async fn export_html(report: &Report, output_path: &Path) -> AnalyticsResult<Vec<u8>> {
        let html_content = format!(
            r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <title>{}</title>
    <style>
        body {{
            font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
            max-width: 1200px;
            margin: 0 auto;
            padding: 20px;
            background-color: #f5f5f5;
        }}
        .header {{
            background-color: #2c3e50;
            color: white;
            padding: 20px;
            border-radius: 5px;
            margin-bottom: 20px;
        }}
        .header h1 {{
            margin: 0;
            font-size: 28px;
        }}
        .metadata {{
            background-color: white;
            padding: 15px;
            border-radius: 5px;
            margin-bottom: 20px;
            box-shadow: 0 2px 4px rgba(0,0,0,0.1);
        }}
        .metadata p {{
            margin: 5px 0;
        }}
        .summary {{
            background-color: #ecf0f1;
            padding: 15px;
            border-left: 4px solid #3498db;
            margin-bottom: 20px;
            border-radius: 3px;
        }}
        .data {{
            background-color: white;
            padding: 20px;
            border-radius: 5px;
            box-shadow: 0 2px 4px rgba(0,0,0,0.1);
        }}
        pre {{
            background-color: #2c3e50;
            color: #ecf0f1;
            padding: 15px;
            border-radius: 3px;
            overflow-x: auto;
        }}
        .footer {{
            text-align: center;
            margin-top: 30px;
            color: #7f8c8d;
            font-size: 12px;
        }}
    </style>
</head>
<body>
    <div class="header">
        <h1>{}</h1>
        <p>Report Type: {:?}</p>
    </div>

    <div class="metadata">
        <p><strong>Report ID:</strong> {}</p>
        <p><strong>Generated:</strong> {}</p>
        <p><strong>Period:</strong> {} to {}</p>
    </div>

    <div class="summary">
        <h2>Summary</h2>
        <p>{}</p>
    </div>

    <div class="data">
        <h2>Report Data</h2>
        <pre>{}</pre>
    </div>

    <div class="footer">
        <p>Generated by LLM Incident Manager Analytics Engine</p>
    </div>
</body>
</html>"#,
            Self::escape_html(&report.title),
            Self::escape_html(&report.title),
            report.report_type,
            Self::escape_html(&report.id),
            report.generated_at.to_rfc3339(),
            report.period_start.to_rfc3339(),
            report.period_end.to_rfc3339(),
            Self::escape_html(&report.summary),
            Self::escape_html(&serde_json::to_string_pretty(&report.data).unwrap_or_default())
        );

        let bytes = html_content.as_bytes().to_vec();

        fs::write(output_path, &bytes)
            .await
            .map_err(|e| AnalyticsError::ExportFailed(format!("Failed to write HTML file: {}", e)))?;

        Ok(bytes)
    }

    /// Export report to bytes without writing to file
    pub async fn export_to_bytes(report: &Report, format: ExportFormat) -> AnalyticsResult<Vec<u8>> {
        match format {
            ExportFormat::Json => {
                let json = serde_json::to_string_pretty(report)
                    .map_err(|e| AnalyticsError::ExportFailed(format!("JSON serialization failed: {}", e)))?;
                Ok(json.as_bytes().to_vec())
            }
            ExportFormat::Csv => {
                // Similar to export_csv but without file writing
                let mut csv_content = String::new();
                csv_content.push_str("Report Title,Report Type,Generated At,Period Start,Period End,Summary\n");
                csv_content.push_str(&format!(
                    "\"{}\",\"{:?}\",\"{}\",\"{}\",\"{}\",\"{}\"\n",
                    Self::escape_csv(&report.title),
                    report.report_type,
                    report.generated_at.to_rfc3339(),
                    report.period_start.to_rfc3339(),
                    report.period_end.to_rfc3339(),
                    Self::escape_csv(&report.summary)
                ));
                Ok(csv_content.as_bytes().to_vec())
            }
            _ => Err(AnalyticsError::UnsupportedFormat(format!(
                "{:?} format not supported for byte export",
                format
            ))),
        }
    }

    /// Escape CSV special characters
    fn escape_csv(s: &str) -> String {
        s.replace('"', "\"\"")
    }

    /// Escape HTML special characters
    fn escape_html(s: &str) -> String {
        s.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&#x27;")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_export_format_extension() {
        assert_eq!(ExportFormat::Json.extension(), "json");
        assert_eq!(ExportFormat::Csv.extension(), "csv");
        assert_eq!(ExportFormat::Pdf.extension(), "pdf");
        assert_eq!(ExportFormat::Html.extension(), "html");
    }

    #[test]
    fn test_export_format_mime_type() {
        assert_eq!(ExportFormat::Json.mime_type(), "application/json");
        assert_eq!(ExportFormat::Csv.mime_type(), "text/csv");
    }

    #[test]
    fn test_escape_csv() {
        assert_eq!(ReportExporter::escape_csv("test\"quote"), "test\"\"quote");
    }

    #[test]
    fn test_escape_html() {
        assert_eq!(ReportExporter::escape_html("<script>"), "&lt;script&gt;");
    }
}
