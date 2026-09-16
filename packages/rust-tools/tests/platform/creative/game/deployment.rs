use super::{call, create_project, TempWorkspace};
use serde_json::json;

fn config(workspace: &TempWorkspace) -> ai_tools::core::config::ServerConfig {
    let mut config = workspace.config();
    config.creative_binding_descriptors = vec![json!({
        "binding_id":"binding_local_game",
        "binding_version":"local-static-game-v1",
        "capabilities":["game.deploy"],
        "media_roles":["deployment"],
        "extension_schema":{"type":"object","additionalProperties":false},
        "constraints":{"estimate":{"base_compute_units":10,"base_output_bytes":4096}},
        "estimate_available":true,
        "availability":"available"
    })
    .to_string()];
    config.creative_binding_backends = vec!["binding_local_game=local_static_game".into()];
    config
}

fn submit_wait(
    config: &ai_tools::core::config::ServerConfig,
    project_id: &str,
    workflow_id: &str,
    parameters: serde_json::Value,
    binding_id: Option<&str>,
) -> serde_json::Value {
    let mut submit = json!({
        "action":"submit",
        "project_id":project_id,
        "workflow_id":workflow_id,
        "parameters":parameters,
        "approved":true
    });
    if let Some(binding_id) = binding_id {
        submit["execution_binding_id"] = json!(binding_id);
    }
    let queued = call(config, "creative_job", submit);
    let job_id = queued["job"]["job_id"].as_str().unwrap().to_owned();
    call(
        config,
        "creative_job",
        json!({"action":"wait","project_id":project_id,"job_id":job_id}),
    )["job"]
        .clone()
}

#[test]
fn local_static_game_binding_creates_private_durable_deployment_without_publish() {
    let workspace = TempWorkspace::new();
    let config = config(&workspace);
    let project_id = "project_local_deploy";
    create_project(&config, project_id);
    call(
        &config,
        "creative_project",
        json!({
            "action":"game_put",
            "project_id":project_id,
            "game":{
                "game_id":"relay_arcade",
                "title":"Relay Arcade",
                "production_intent":"build",
                "genre":"arcade",
                "perspective":"top_down",
                "core_loop":"Move and collect points.",
                "win_condition":"Reach ten points.",
                "lose_condition":"Lose all lives.",
                "restart_behavior":"Press R to restart.",
                "player_mode":"solo",
                "target_devices":["desktop"],
                "verbs":["move","collect","restart"],
                "inputs":["keyboard"],
                "asset_roles":[]
            }
        }),
    );

    assert_eq!(
        submit_wait(
            &config,
            project_id,
            "game_source_scaffold",
            json!({"game_id":"relay_arcade","template_family":"2d_canvas"}),
            None,
        )["status"],
        "completed"
    );
    let build = submit_wait(
        &config,
        project_id,
        "game_build_playtest",
        json!({"game_id":"relay_arcade"}),
        None,
    );
    assert_eq!(build["status"], "completed");
    let build_revision = build["node_runs"][0]["output"]["build_revision"]
        .as_str()
        .unwrap()
        .to_owned();

    let deploy = submit_wait(
        &config,
        project_id,
        "game_deploy",
        json!({"game_id":"relay_arcade","build_revision":build_revision}),
        Some("binding_local_game"),
    );
    assert_eq!(deploy["status"], "completed");
    assert_eq!(deploy["node_runs"][0]["output"]["published"], false);
    let deployment_id = deploy["node_runs"][0]["output"]["deployment_id"]
        .as_str()
        .unwrap();
    assert_eq!(
        deploy["node_runs"][0]["output"]["deployment_url"],
        format!("/creative-deploy/{deployment_id}/index.html")
    );
    assert!(workspace
        .path(&format!(
            ".masihawam/creative/deployments/{deployment_id}/site/index.html"
        ))
        .is_file());
    assert!(workspace
        .path(&format!(
            ".masihawam/creative/deployments/{deployment_id}/site/game.js"
        ))
        .is_file());

    let game = call(
        &config,
        "creative_project",
        json!({"action":"game_get","project_id":project_id,"game_id":"relay_arcade"}),
    );
    assert_eq!(game["game"]["build_state"]["deployment_id"], deployment_id);
    assert_eq!(game["game"]["build_state"]["published"], false);
}

#[test]
fn local_static_game_deployment_is_served_through_the_relay_access_boundary() {
    use ai_tools::infrastructure::transport::create_router;
    use std::time::Duration;

    let workspace = TempWorkspace::new();
    let mut config = config(&workspace);
    config.origin = Some("http://localhost:3333".into());
    let project_id = "project_local_deploy_http";
    create_project(&config, project_id);
    call(
        &config,
        "creative_project",
        json!({
            "action":"game_put",
            "project_id":project_id,
            "game":{
                "game_id":"relay_http_arcade",
                "title":"Relay HTTP Arcade",
                "production_intent":"build",
                "genre":"arcade",
                "perspective":"top_down",
                "core_loop":"Move and collect points.",
                "win_condition":"Reach ten points.",
                "lose_condition":"Lose all lives.",
                "restart_behavior":"Press R to restart.",
                "player_mode":"solo",
                "target_devices":["desktop"],
                "verbs":["move","collect","restart"],
                "inputs":["keyboard"],
                "asset_roles":[]
            }
        }),
    );
    assert_eq!(
        submit_wait(
            &config,
            project_id,
            "game_source_scaffold",
            json!({"game_id":"relay_http_arcade","template_family":"2d_canvas"}),
            None,
        )["status"],
        "completed"
    );
    let build = submit_wait(
        &config,
        project_id,
        "game_build_playtest",
        json!({"game_id":"relay_http_arcade"}),
        None,
    );
    let build_revision = build["node_runs"][0]["output"]["build_revision"]
        .as_str()
        .unwrap()
        .to_owned();
    let deploy = submit_wait(
        &config,
        project_id,
        "game_deploy",
        json!({"game_id":"relay_http_arcade","build_revision":build_revision}),
        Some("binding_local_game"),
    );
    let deployment_url = deploy["node_runs"][0]["output"]["deployment_url"]
        .as_str()
        .unwrap()
        .to_owned();

    let runtime = tokio::runtime::Runtime::new().expect("creative deploy HTTP runtime");
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
                .expect("creative deploy test server");
        });
        (port, server)
    });

    runtime.block_on(async {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .expect("test HTTP client");
        let response = client
            .get(format!("http://127.0.0.1:{port}{deployment_url}"))
            .header("origin", "http://localhost:3333")
            .send()
            .await
            .expect("deployment response");
        assert_eq!(response.status(), reqwest::StatusCode::OK);
        assert_eq!(
            response
                .headers()
                .get(reqwest::header::CACHE_CONTROL)
                .and_then(|value| value.to_str().ok()),
            Some("private, no-store")
        );
        assert!(response
            .headers()
            .get(reqwest::header::CONTENT_SECURITY_POLICY)
            .is_some());
        let html = response.text().await.expect("deployment html");
        assert!(html.contains("Relay HTTP Arcade"));
    });
    server.abort();
}
