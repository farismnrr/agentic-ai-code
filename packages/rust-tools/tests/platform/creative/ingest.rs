use super::{call, create_project, TempWorkspace};
use ai_tools::infrastructure::transport::create_router;
use serde_json::Value;
use std::time::Duration;

#[test]
fn upload_handoff_is_bounded_single_use_and_completes_to_durable_asset() {
    let workspace = TempWorkspace::new();
    let mut config = workspace.config();
    config.origin = Some("http://localhost:3333".into());
    create_project(&config, "project_upload");

    let requested = call(
        &config,
        "creative_asset",
        serde_json::json!({
            "action": "upload_request",
            "project_id": "project_upload",
            "media_type": "image/png",
            "role": "character_reference",
            "filename": "hero.png",
            "max_bytes": 1024,
            "ttl_ms": 60_000
        }),
    );
    let upload = &requested["upload"];
    let path = upload["upload_path"]
        .as_str()
        .expect("upload path")
        .to_owned();
    let token = upload["upload_token"]
        .as_str()
        .expect("upload token")
        .to_owned();
    assert!(!token.is_empty());
    assert!(!path.contains(&token));

    let runtime = tokio::runtime::Runtime::new().expect("creative upload HTTP runtime");
    let (port, server) = runtime.block_on(async {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("loopback listener");
        let port = listener.local_addr().expect("listener address").port();
        config.port = port;
        let router = create_router(config.clone());
        let server = tokio::spawn(async move {
            axum::serve(listener, router)
                .await
                .expect("creative upload test server");
        });
        (port, server)
    });

    runtime.block_on(async {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .expect("test HTTP client");
        let upload_url = format!("http://127.0.0.1:{port}{path}");
        let uploaded = client
            .put(&upload_url)
            .header("origin", "http://localhost:3333")
            .header("content-type", "image/png")
            .header("x-creative-upload-token", &token)
            .body(b"fixture-png-bytes".to_vec())
            .send()
            .await
            .expect("upload response");
        assert_eq!(uploaded.status(), reqwest::StatusCode::CREATED);
        let receipt: Value = uploaded.json().await.expect("upload receipt");
        assert_eq!(receipt["upload"]["project_id"], "project_upload");
        assert_eq!(receipt["upload"]["media_type"], "image/png");
        assert_eq!(receipt["upload"]["asset_id"], Value::Null);

        let replay = client
            .put(&upload_url)
            .header("origin", "http://localhost:3333")
            .header("content-type", "image/png")
            .header("x-creative-upload-token", &token)
            .body(b"fixture-png-bytes".to_vec())
            .send()
            .await
            .expect("replay response");
        assert_eq!(replay.status(), reqwest::StatusCode::BAD_REQUEST);
    });

    let completed = call(
        &config,
        "creative_asset",
        serde_json::json!({
            "action": "upload_complete",
            "project_id": "project_upload",
            "ticket_id": upload["ticket_id"]
        }),
    );
    let asset_id = completed["asset_id"].as_str().expect("asset id").to_owned();
    assert_eq!(completed["asset"]["source"], "mcp_upload");
    assert_eq!(completed["asset"]["source_surface"], "mcp");
    assert_eq!(completed["asset"]["state"], "candidate");
    assert_eq!(completed["preview"]["asset_id"], asset_id);
    assert_eq!(
        completed["preview"]["checksum_sha256"]
            .as_str()
            .map(str::len),
        Some(64)
    );

    let completed_again = call(
        &config,
        "creative_asset",
        serde_json::json!({
            "action": "upload_complete",
            "project_id": "project_upload",
            "ticket_id": upload["ticket_id"]
        }),
    );
    assert_eq!(completed_again["asset_id"], asset_id);

    server.abort();
    runtime.block_on(async {
        let _ = server.await;
    });
}

#[test]
fn upload_and_import_schemas_reject_incomplete_or_unsafe_inputs() {
    let workspace = TempWorkspace::new();
    let config = workspace.config();
    create_project(&config, "project_ingest_validation");

    let bad_filename = super::dispatch_sync(
        &config,
        "creative_asset",
        &serde_json::json!({
            "action": "upload_request",
            "project_id": "project_ingest_validation",
            "media_type": "image/png",
            "role": "reference",
            "filename": "../escape.png"
        }),
    );
    assert!(bad_filename.is_err());

    let conversation = call(
        &config,
        "creative_asset",
        serde_json::json!({
            "action": "upload_request",
            "project_id": "project_ingest_validation",
            "source": "conversation_upload",
            "media_type": "image/png",
            "role": "conversation_reference",
            "filename": "conversation.png"
        }),
    );
    assert!(conversation["upload"]["ticket_id"].as_str().is_some());

    let listed = call(
        &config,
        "creative_asset",
        serde_json::json!({
            "action": "upload_list",
            "project_id": "project_ingest_validation"
        }),
    );
    assert!(listed["uploads"].as_array().unwrap().iter().any(|ticket| {
        ticket["source"] == "conversation_upload"
            && ticket["role"] == "conversation_reference"
            && ticket["token_sha256"] == ""
    }));

    let forged_source = super::dispatch_sync(
        &config,
        "creative_asset",
        &serde_json::json!({
            "action": "upload_request",
            "project_id": "project_ingest_validation",
            "source": "manual_import",
            "media_type": "image/png",
            "role": "reference",
            "filename": "forged.png"
        }),
    );
    assert!(forged_source.is_err());

    let private_url = super::dispatch_sync(
        &config,
        "creative_asset",
        &serde_json::json!({
            "action": "import_url",
            "project_id": "project_ingest_validation",
            "url": "http://127.0.0.1/private.png",
            "role": "reference"
        }),
    );
    assert!(private_url.is_err());
}
