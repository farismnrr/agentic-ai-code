//! OpenTelemetry plumbing for the Rust relay/CLI processes.
//!
//! Scope (Plan 035 Phase 8): foundation only. This module wires up an OTLP-gRPC
//! trace pipeline behind an opt-in env var, bridges it into `tracing`, and exposes
//! a bounded-shutdown flush helper. It does NOT instrument any request/auth/
//! admission/tool spans — that is deliberately deferred to a later phase.
//!
//! The existing stderr JSON audit log in `observability.rs` (`audit()`,
//! `safe_log_field`, `MAX_LOG_FIELD`, `CorrelationId`) is left completely
//! untouched and remains the source of truth for the audit trail. This module
//! is purely additive.
//!
//! ## Env vars
//! - `NUXT_OTEL_ENABLED` (bool, default `false`): opt-in switch. Reused verbatim
//!   from the Nuxt side (`otel-preload.mjs`) so operators configure one set of
//!   vars for both the Node and Rust processes.
//! - `NUXT_OTEL_JAEGER_ENDPOINT` (default `http://localhost:4317`): OTLP-gRPC
//!   collector endpoint. Same var as the Nuxt side.
//! - `NUXT_OTEL_SERVICE_NAME`: informational only on the Rust side (the Rust
//!   service identity is fixed below); accepted for symmetry but not required.
//! - `NODE_ENV`: reused (again mirroring the Nuxt side) to populate
//!   `deployment.environment`. Defaults to `"development"` when unset.
//!
//! `service.name` is a fixed literal (`"ai-code-relay"`) rather than reading
//! `NUXT_OTEL_SERVICE_NAME`, so relay traces are always distinguishable from
//! the Nuxt server's traces in a shared collector regardless of operator env
//! configuration drift. `service.version` comes from `CARGO_PKG_VERSION` of the
//! crate being compiled (here, `relay-infrastructure`, which inherits
//! `workspace.package.version` via `version.workspace = true`).
//!
//! ## Deferred
//! A full `tracing-subscriber` fmt-to-OTel **logs** bridge is deferred: only
//! spans/traces are wired in this phase (per task priority). Adding OTel log
//! export would require the `logs` feature end-to-end plus a `tracing-log`
//! bridge layer, which is unnecessary complexity while no application code
//! emits `tracing` events yet beyond the one smoke-test span in `main.rs`.

use opentelemetry::global;
use opentelemetry::propagation::{Extractor, TextMapPropagator};
use opentelemetry::trace::TracerProvider as _;
use opentelemetry::Context;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::propagation::TraceContextPropagator;
use opentelemetry_sdk::trace::{BatchConfigBuilder, BatchSpanProcessor, SdkTracerProvider};
use opentelemetry_sdk::Resource;
use std::time::Duration;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

/// Attribute/field value cap for OTel spans. Distinct from (and larger than)
/// `observability::MAX_LOG_FIELD` (128, unchanged) which governs the stderr
/// audit log; 256 gives OTel attributes a bit more room since they are
/// consumed by a backend rather than grepped by humans.
pub const MAX_OTEL_FIELD: usize = 256;

const HDR_ENABLED: &str = "NUXT_OTEL_ENABLED";
const HDR_ENDPOINT: &str = "NUXT_OTEL_JAEGER_ENDPOINT";
const DEFAULT_ENDPOINT: &str = "http://localhost:4317";
const EXPORT_TIMEOUT: Duration = Duration::from_secs(5);
const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(3);

/// Handle returned by [`init_telemetry`]. Dropping it does nothing special;
/// callers must explicitly call [`shutdown_telemetry`] with a bounded timeout
/// before process exit to flush any buffered spans.
pub struct TelemetryGuard {
    provider: SdkTracerProvider,
}

fn env_enabled() -> bool {
    std::env::var(HDR_ENABLED)
        .map(|v| matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on"))
        .unwrap_or(false)
}

/// Initialize the OTel trace pipeline + tracing-subscriber bridge, if enabled.
///
/// Reads `NUXT_OTEL_ENABLED` (default off) and `NUXT_OTEL_JAEGER_ENDPOINT`
/// (default `http://localhost:4317`, OTLP-gRPC). When disabled, this is a
/// no-op: returns `None`, installs no global subscriber, and nothing else in
/// the process behaves differently.
///
/// When enabled, also installs the global W3C `traceparent` propagator and a
/// `tracing_subscriber` global default subscriber bridging `tracing` spans
/// into OTel spans via `tracing-opentelemetry`.
pub fn init_telemetry() -> Option<TelemetryGuard> {
    if !env_enabled() {
        return None;
    }

    let endpoint = std::env::var(HDR_ENDPOINT).unwrap_or_else(|_| DEFAULT_ENDPOINT.to_string());
    let environment = std::env::var("NODE_ENV").unwrap_or_else(|_| "development".to_string());

    let exporter = match opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_endpoint(&endpoint)
        .with_timeout(EXPORT_TIMEOUT)
        .build()
    {
        Ok(exporter) => exporter,
        Err(err) => {
            eprintln!("telemetry: failed to build OTLP exporter, disabling: {err}");
            return None;
        }
    };

    // OTel SDK default-shaped batch processor: 512 spans/batch, 2048 queue
    // size. We set these explicitly to document the contract rather than
    // hand-rolling a custom batcher.
    let batch_config = BatchConfigBuilder::default()
        .with_max_export_batch_size(512)
        .with_max_queue_size(2048)
        .build();
    let processor = BatchSpanProcessor::builder(exporter)
        .with_batch_config(batch_config)
        .build();

    let resource = Resource::builder()
        .with_service_name("ai-code-relay")
        .with_attribute(opentelemetry::KeyValue::new(
            "service.version",
            env!("CARGO_PKG_VERSION"),
        ))
        .with_attribute(opentelemetry::KeyValue::new(
            "deployment.environment",
            environment,
        ))
        .build();

    let provider = SdkTracerProvider::builder()
        .with_span_processor(processor)
        .with_resource(resource)
        .build();

    global::set_text_map_propagator(TraceContextPropagator::new());
    global::set_tracer_provider(provider.clone());

    let tracer = provider.tracer("ai-code-relay");
    let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);

    if let Err(err) = tracing_subscriber::registry().with(otel_layer).try_init() {
        eprintln!("telemetry: failed to install tracing subscriber: {err}");
    }

    Some(TelemetryGuard { provider })
}

/// Bounded-timeout flush + shutdown of the tracer provider. Never panics and
/// never hangs the process: if shutdown doesn't complete within `timeout`
/// (recommended 2-3s), a message is logged to stderr and the function
/// returns anyway.
///
/// No-op when `guard` is `None` (telemetry disabled).
pub async fn shutdown_telemetry(guard: Option<TelemetryGuard>, timeout: Duration) {
    let Some(guard) = guard else { return };
    let shutdown = tokio::task::spawn_blocking(move || guard.provider.shutdown());
    match tokio::time::timeout(timeout, shutdown).await {
        Ok(Ok(Ok(()))) => {}
        Ok(Ok(Err(err))) => eprintln!("telemetry: shutdown error: {err}"),
        Ok(Err(err)) => eprintln!("telemetry: shutdown task join error: {err}"),
        Err(_) => eprintln!("telemetry: shutdown timed out after {timeout:?}, continuing"),
    }
}

/// Convenience wrapper using the recommended default shutdown timeout (3s).
pub async fn shutdown_telemetry_default(guard: Option<TelemetryGuard>) {
    shutdown_telemetry(guard, SHUTDOWN_TIMEOUT).await;
}

/// Thin adapter so `axum::http::HeaderMap` can be used as an
/// `opentelemetry::propagation::Extractor` for W3C `traceparent` extraction.
struct HeaderExtractor<'a>(&'a axum::http::HeaderMap);

impl<'a> Extractor for HeaderExtractor<'a> {
    fn get(&self, key: &str) -> Option<&str> {
        self.0.get(key).and_then(|v| v.to_str().ok())
    }

    fn keys(&self) -> Vec<&str> {
        self.0.keys().map(|k| k.as_str()).collect()
    }
}

/// Extract a W3C Trace Context (`traceparent`/`tracestate`) from inbound
/// request headers using the globally installed propagator.
///
/// Not yet wired into `transport.rs` request handling — this phase only
/// makes the helper available for a later instrumentation phase.
pub fn extract_traceparent(headers: &axum::http::HeaderMap) -> Context {
    let propagator = TraceContextPropagator::new();
    propagator.extract(&HeaderExtractor(headers))
}
