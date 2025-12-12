//! OTLP (OpenTelemetry Protocol) export for telemetry.
//!
//! Converts TelemetryContext to OTLP format and exports via HTTP.
//! This module is only compiled when the `otlp` feature is enabled.

#![cfg(feature = "otlp")]

use anyhow::{Context, Result};
use opentelemetry_proto::tonic::collector::trace::v1::ExportTraceServiceRequest;
use opentelemetry_proto::tonic::common::v1::{any_value, AnyValue, KeyValue};
use opentelemetry_proto::tonic::resource::v1::Resource;
use opentelemetry_proto::tonic::trace::v1::{
    span::SpanKind, ResourceSpans, ScopeSpans, Span, Status,
};
use prost::Message;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{debug, warn};

use super::context::TelemetryContext;

/// Service name for OTLP resource
const SERVICE_NAME: &str = "cupcake";
const INSTRUMENTATION_SCOPE_NAME: &str = "cupcake.telemetry";

/// Export telemetry to an OTLP HTTP endpoint.
///
/// Converts the TelemetryContext to OTLP format and sends via HTTP POST.
pub async fn export_otlp(ctx: &TelemetryContext, endpoint: &str) -> Result<()> {
    let request = to_export_request(ctx);
    let payload = request.encode_to_vec();

    let url = format!("{}/v1/traces", endpoint.trim_end_matches('/'));
    debug!("Exporting OTLP traces to {}", url);

    let client = reqwest::Client::new();
    let response = client
        .post(&url)
        .header("Content-Type", "application/x-protobuf")
        .body(payload)
        .send()
        .await
        .context("Failed to send OTLP export request")?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        warn!("OTLP export failed: {} - {}", status, body);
        anyhow::bail!("OTLP export failed with status {}", status);
    }

    debug!("OTLP export successful");
    Ok(())
}

/// Convert TelemetryContext to OTLP ExportTraceServiceRequest.
fn to_export_request(ctx: &TelemetryContext) -> ExportTraceServiceRequest {
    let spans = build_spans(ctx);

    let scope_spans = ScopeSpans {
        scope: Some(opentelemetry_proto::tonic::common::v1::InstrumentationScope {
            name: INSTRUMENTATION_SCOPE_NAME.to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            ..Default::default()
        }),
        spans,
        ..Default::default()
    };

    let resource_spans = ResourceSpans {
        resource: Some(Resource {
            attributes: vec![
                kv_string("service.name", SERVICE_NAME),
                kv_string("service.version", env!("CARGO_PKG_VERSION")),
                kv_string("telemetry.sdk.name", "cupcake"),
                kv_string("telemetry.sdk.language", "rust"),
            ],
            ..Default::default()
        }),
        scope_spans: vec![scope_spans],
        ..Default::default()
    };

    ExportTraceServiceRequest {
        resource_spans: vec![resource_spans],
    }
}

/// Build OTLP spans from TelemetryContext.
fn build_spans(ctx: &TelemetryContext) -> Vec<Span> {
    let mut spans = Vec::new();
    let trace_id = parse_trace_id(&ctx.ingest.trace_id);

    // 1. Root span: cupcake.ingest
    let ingest_span_id = parse_span_id(&ctx.ingest.span_id);
    let start_time_ns = system_time_to_nanos(&ctx.ingest.timestamp);
    let end_time_ns = start_time_ns + (ctx.total_duration_ms * 1_000_000);

    let mut ingest_attrs = vec![
        kv_string("harness", &format!("{:?}", ctx.ingest.harness)),
        kv_string("trace_id", &ctx.ingest.trace_id),
    ];

    // Add raw event as JSON attribute
    if let Ok(raw_json) = serde_json::to_string(&ctx.ingest.raw_event) {
        ingest_attrs.push(kv_string("raw_event", &raw_json));
    }

    spans.push(Span {
        trace_id: trace_id.to_vec(),
        span_id: ingest_span_id.to_vec(),
        parent_span_id: vec![], // Root span has no parent
        name: "cupcake.ingest".to_string(),
        kind: SpanKind::Server as i32,
        start_time_unix_nano: start_time_ns,
        end_time_unix_nano: end_time_ns,
        attributes: ingest_attrs,
        status: Some(Status {
            code: if ctx.errors.is_empty() { 1 } else { 2 }, // 1=Ok, 2=Error
            message: ctx.errors.first().cloned().unwrap_or_default(),
        }),
        ..Default::default()
    });

    // 2. Child span: cupcake.enrich (if present)
    if let Some(ref enrich) = ctx.enrich {
        let enrich_span_id = parse_span_id(&enrich.span_id);
        let enrich_parent_id = parse_span_id(&enrich.parent_span_id);
        let enrich_start = start_time_ns;
        let enrich_end = enrich_start + (enrich.duration_us * 1000); // us to ns

        let mut enrich_attrs = vec![kv_string(
            "operations",
            &enrich.preprocessing_operations.join(", "),
        )];

        if let Ok(enriched_json) = serde_json::to_string(&enrich.enriched_event) {
            enrich_attrs.push(kv_string("enriched_event", &enriched_json));
        }

        spans.push(Span {
            trace_id: trace_id.to_vec(),
            span_id: enrich_span_id.to_vec(),
            parent_span_id: enrich_parent_id.to_vec(),
            name: "cupcake.enrich".to_string(),
            kind: SpanKind::Internal as i32,
            start_time_unix_nano: enrich_start,
            end_time_unix_nano: enrich_end,
            attributes: enrich_attrs,
            status: Some(Status {
                code: 1, // Ok
                message: String::new(),
            }),
            ..Default::default()
        });
    }

    // 3. Child spans: cupcake.evaluate (one per phase)
    let mut eval_start_offset = ctx.enrich.as_ref().map(|e| e.duration_us * 1000).unwrap_or(0);

    for eval in &ctx.evaluations {
        let eval_span_id = parse_span_id(&eval.span_id);
        let eval_parent_id = parse_span_id(&eval.parent_span_id);
        let eval_start = start_time_ns + eval_start_offset;
        let eval_end = eval_start + (eval.duration_ms * 1_000_000);

        let mut eval_attrs = vec![
            kv_string("phase", &eval.phase),
            kv_bool("routed", eval.routed),
        ];

        if !eval.matched_policies.is_empty() {
            eval_attrs.push(kv_string(
                "matched_policies",
                &eval.matched_policies.join(", "),
            ));
        }

        if let Some(ref exit_reason) = eval.exit_reason {
            eval_attrs.push(kv_string("exit_reason", exit_reason));
        }

        if let Some(ref decision) = eval.final_decision {
            eval_attrs.push(kv_string("final_decision", &format!("{:?}", decision)));
        }

        if let Some(ref decision_set) = eval.wasm_decision_set {
            eval_attrs.push(kv_int("halts", decision_set.halts.len() as i64));
            eval_attrs.push(kv_int("denials", decision_set.denials.len() as i64));
            eval_attrs.push(kv_int("blocks", decision_set.blocks.len() as i64));
            eval_attrs.push(kv_int("asks", decision_set.asks.len() as i64));
        }

        spans.push(Span {
            trace_id: trace_id.to_vec(),
            span_id: eval_span_id.to_vec(),
            parent_span_id: eval_parent_id.to_vec(),
            name: format!("cupcake.evaluate.{}", eval.phase),
            kind: SpanKind::Internal as i32,
            start_time_unix_nano: eval_start,
            end_time_unix_nano: eval_end,
            attributes: eval_attrs,
            status: Some(Status {
                code: 1, // Ok
                message: String::new(),
            }),
            ..Default::default()
        });

        eval_start_offset += eval.duration_ms * 1_000_000;
    }

    spans
}

// Helper functions for creating KeyValue attributes
fn kv_string(key: &str, value: &str) -> KeyValue {
    KeyValue {
        key: key.to_string(),
        value: Some(AnyValue {
            value: Some(any_value::Value::StringValue(value.to_string())),
        }),
    }
}

fn kv_bool(key: &str, value: bool) -> KeyValue {
    KeyValue {
        key: key.to_string(),
        value: Some(AnyValue {
            value: Some(any_value::Value::BoolValue(value)),
        }),
    }
}

fn kv_int(key: &str, value: i64) -> KeyValue {
    KeyValue {
        key: key.to_string(),
        value: Some(AnyValue {
            value: Some(any_value::Value::IntValue(value)),
        }),
    }
}

/// Parse trace_id string to 16-byte array (OTLP requires 16 bytes).
/// Uses first 16 bytes of hex-decoded trace_id, padding with zeros if needed.
fn parse_trace_id(trace_id: &str) -> [u8; 16] {
    let mut result = [0u8; 16];
    // Try to decode as hex first
    if let Ok(bytes) = hex::decode(trace_id.replace('-', "")) {
        let len = bytes.len().min(16);
        result[..len].copy_from_slice(&bytes[..len]);
    } else {
        // Fall back to using hash of the string
        use sha2::{Digest, Sha256};
        let hash = Sha256::digest(trace_id.as_bytes());
        result.copy_from_slice(&hash[..16]);
    }
    result
}

/// Parse span_id string to 8-byte array (OTLP requires 8 bytes).
fn parse_span_id(span_id: &str) -> [u8; 8] {
    let mut result = [0u8; 8];
    if let Ok(bytes) = hex::decode(span_id) {
        let len = bytes.len().min(8);
        result[..len].copy_from_slice(&bytes[..len]);
    }
    result
}

/// Convert SystemTime to nanoseconds since Unix epoch.
fn system_time_to_nanos(time: &SystemTime) -> u64 {
    time.duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::harness::types::HarnessType;
    use serde_json::json;

    #[test]
    fn test_span_id_parsing() {
        let span_id = "abc123def4567890";
        let parsed = parse_span_id(span_id);
        assert_eq!(parsed.len(), 8);
        assert_eq!(hex::encode(parsed), span_id);
    }

    #[test]
    fn test_trace_id_parsing() {
        // UUID v7 format
        let trace_id = "019b0a92-e8b9-7781-889f-4e47252d167a";
        let parsed = parse_trace_id(trace_id);
        assert_eq!(parsed.len(), 16);
        // Should have parsed the hex bytes
        assert_ne!(parsed, [0u8; 16]);
    }

    #[test]
    fn test_build_spans() {
        let raw = json!({"hook_event_name": "PreToolUse"});
        let ctx = TelemetryContext::new(raw, HarnessType::ClaudeCode, "test-trace-123".into());

        let spans = build_spans(&ctx);
        assert_eq!(spans.len(), 1); // Just ingest span

        assert_eq!(spans[0].name, "cupcake.ingest");
        assert!(spans[0].parent_span_id.is_empty()); // Root has no parent
    }

    #[test]
    fn test_build_spans_with_evaluation() {
        let raw = json!({"hook_event_name": "PreToolUse"});
        let mut ctx = TelemetryContext::new(raw, HarnessType::ClaudeCode, "test-trace-123".into());

        ctx.record_enrichment(json!({"enriched": true}), vec!["op1".into()], 100);

        let eval = ctx.start_evaluation("project");
        eval.record_routing(true, &["policy.test".into()]);
        eval.finalize();

        let spans = build_spans(&ctx);
        assert_eq!(spans.len(), 3); // ingest + enrich + evaluate

        assert_eq!(spans[0].name, "cupcake.ingest");
        assert_eq!(spans[1].name, "cupcake.enrich");
        assert_eq!(spans[2].name, "cupcake.evaluate.project");

        // Verify parent relationships
        let ingest_id = &spans[0].span_id;
        assert_eq!(&spans[1].parent_span_id, ingest_id);
        assert_eq!(&spans[2].parent_span_id, ingest_id);
    }
}
