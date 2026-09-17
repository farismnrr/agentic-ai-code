use super::{call, create_project, TempWorkspace};
use ai_tools::application::creative::dispatch_tool;
use ai_tools::core::config::SecurityMode;
use ai_tools::core::error::McpError;
use ai_tools::infrastructure::transport::create_router;
use serde_json::Value;
use std::net::SocketAddr;
use std::time::Duration;

fn call_for_owner(
    config: &ai_tools::core::config::ServerConfig,
    name: &str,
    arguments: serde_json::Value,
    owner: &str,
) -> Value {
    let result = tokio::runtime::Runtime::new()
        .expect("creative owner test runtime")
        .block_on(dispatch_tool(name, &arguments, config, owner))
        .expect("creative owner dispatch")
        .expect("creative owner tool result");
    assert!(!result.is_error, "creative owner tool returned an error");
    serde_json::from_str(&result.content[0].text).expect("creative owner JSON result")
}

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
fn remote_upload_uses_the_ticket_capability_without_forwarding_mcp_oauth() {
    let workspace = TempWorkspace::new();
    let owner = "remote-upload-owner";
    let project_id = "project_remote_upload";
    let mut config = workspace.config();
    config.mode = SecurityMode::Remote;
    config.origin = Some("https://chat.example.test".into());
    config.bind_host = "0.0.0.0".into();
    config.trusted_proxy = true;
    config.trusted_proxy_cidr = Some("127.0.0.1/32".into());
    config.oauth_issuer = Some("https://issuer.example.test/realms/creative".into());
    config.oauth_audience = Some("https://mcp.example.test/mcp".into());
    config.oauth_owner_subject = Some(owner.into());

    let created = call_for_owner(
        &config,
        "creative_project",
        serde_json::json!({
            "action":"create",
            "project_id":project_id,
            "title":"Remote upload fixture",
            "intent":"Prove bearerless single-use upload capability",
            "tracks":["scene","anime","game"]
        }),
        owner,
    );
    assert_eq!(created["project"]["project_id"], project_id);

    let requested = call_for_owner(
        &config,
        "creative_asset",
        serde_json::json!({
            "action":"upload_request",
            "project_id":project_id,
            "media_type":"image/svg+xml",
            "role":"external_parity_fixture",
            "filename":"external.svg",
            "max_bytes":2048,
            "ttl_ms":60_000
        }),
        owner,
    );
    let upload = &requested["upload"];
    let path = upload["upload_path"].as_str().expect("upload path");
    let ticket_id = upload["ticket_id"].as_str().expect("ticket ID");
    let token = upload["upload_token"].as_str().expect("upload token");
    let expired = call_for_owner(
        &config,
        "creative_asset",
        serde_json::json!({
            "action":"upload_request",
            "project_id":project_id,
            "media_type":"image/svg+xml",
            "role":"external_parity_fixture",
            "filename":"expired.svg",
            "max_bytes":2048,
            "ttl_ms":1
        }),
        owner,
    );
    let expired_upload = &expired["upload"];
    let expired_path = expired_upload["upload_path"]
        .as_str()
        .expect("expired path");
    let expired_token = expired_upload["upload_token"]
        .as_str()
        .expect("expired upload token");
    std::thread::sleep(Duration::from_millis(5));

    let runtime = tokio::runtime::Runtime::new().expect("remote upload HTTP runtime");
    let (port, server) = runtime.block_on(async {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("remote loopback listener");
        let port = listener.local_addr().expect("listener address").port();
        config.port = port;
        let router = create_router(config.clone());
        let server = tokio::spawn(async move {
            axum::serve(
                listener,
                router.into_make_service_with_connect_info::<SocketAddr>(),
            )
            .await
            .expect("remote creative upload test server");
        });
        (port, server)
    });

    runtime.block_on(async {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .expect("remote upload test client");
        let upload_url = format!("http://127.0.0.1:{port}{path}");
        let origin = "https://chat.example.test";

        let preflight = client
            .request(reqwest::Method::OPTIONS, &upload_url)
            .header("origin", origin)
            .header("x-forwarded-proto", "https")
            .header("access-control-request-method", "PUT")
            .header(
                "access-control-request-headers",
                "content-type, x-creative-upload-token",
            )
            .send()
            .await
            .expect("upload CORS preflight");
        assert!(preflight.status().is_success());
        assert!(preflight
            .headers()
            .get("access-control-allow-methods")
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| value.contains("PUT")));
        assert!(preflight
            .headers()
            .get("access-control-allow-headers")
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| value.contains("x-creative-upload-token")));

        let missing_token = client
            .put(&upload_url)
            .header("origin", origin)
            .header("x-forwarded-proto", "https")
            .header("content-type", "image/svg+xml")
            .body("<svg/>")
            .send()
            .await
            .expect("missing capability response");
        assert_eq!(missing_token.status(), reqwest::StatusCode::UNAUTHORIZED);

        let invalid_token = client
            .put(&upload_url)
            .header("origin", origin)
            .header("x-forwarded-proto", "https")
            .header("content-type", "image/svg+xml")
            .header("x-creative-upload-token", "wrong-capability")
            .body("<svg/>")
            .send()
            .await
            .expect("invalid capability response");
        assert_eq!(invalid_token.status(), reqwest::StatusCode::BAD_REQUEST);

        let expired_url = format!("http://127.0.0.1:{port}{expired_path}");
        let expired_upload = client
            .put(&expired_url)
            .header("origin", origin)
            .header("x-forwarded-proto", "https")
            .header("content-type", "image/svg+xml")
            .header("x-creative-upload-token", expired_token)
            .body("<svg/>")
            .send()
            .await
            .expect("expired capability response");
        assert_eq!(expired_upload.status(), reqwest::StatusCode::BAD_REQUEST);

        let malformed_bearer = client
            .put(&upload_url)
            .header("origin", origin)
            .header("x-forwarded-proto", "https")
            .header("authorization", "Bearer malformed")
            .header("content-type", "image/svg+xml")
            .header("x-creative-upload-token", token)
            .body("<svg/>")
            .send()
            .await
            .expect("malformed bearer response");
        assert_eq!(malformed_bearer.status(), reqwest::StatusCode::UNAUTHORIZED);

        let uploaded = client
            .put(&upload_url)
            .header("origin", origin)
            .header("x-forwarded-proto", "https")
            .header("content-type", "image/svg+xml")
            .header("x-creative-upload-token", token)
            .body("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"1\" height=\"1\"/>")
            .send()
            .await
            .expect("bearerless upload response");
        assert_eq!(uploaded.status(), reqwest::StatusCode::CREATED);

        let replay = client
            .put(&upload_url)
            .header("origin", origin)
            .header("x-forwarded-proto", "https")
            .header("content-type", "image/svg+xml")
            .header("x-creative-upload-token", token)
            .body("<svg/>")
            .send()
            .await
            .expect("replayed capability response");
        assert_eq!(replay.status(), reqwest::StatusCode::BAD_REQUEST);
    });

    let wrong_owner = tokio::runtime::Runtime::new()
        .expect("wrong-owner completion runtime")
        .block_on(dispatch_tool(
            "creative_asset",
            &serde_json::json!({
                "action":"upload_complete",
                "project_id":project_id,
                "ticket_id":ticket_id
            }),
            &config,
            "another-owner",
        ));
    assert!(matches!(wrong_owner, Err(McpError::InvalidRequest(_))));

    let completed = call_for_owner(
        &config,
        "creative_asset",
        serde_json::json!({
            "action":"upload_complete",
            "project_id":project_id,
            "ticket_id":ticket_id
        }),
        owner,
    );
    assert_eq!(completed["asset"]["source"], "mcp_upload");
    assert_eq!(completed["asset"]["state"], "candidate");

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
