use super::super::contracts::{
    AssetMetadata, AssetSource, AssetState, AssetSurface, GameManifest, QaFinding, QaSeverity,
};
use super::super::graph::{CreativeJobRecord, CreativeJobStatus, NodeRunRecord};
use super::super::store::{self, AssetRegistrationInput};
use crate::core::config::ServerConfig;
use crate::core::error::McpError;
use serde_json::{json, Value};
use std::fs;
use uuid::Uuid;

pub(super) fn execute_local(
    cwd: Option<&str>,
    config: &ServerConfig,
    mut job: CreativeJobRecord,
) -> Result<CreativeJobRecord, McpError> {
    let workflow_id = job
        .workflow_id
        .as_deref()
        .ok_or_else(|| McpError::InvalidRequest("game workflow id is required".into()))?;
    let result = match workflow_id {
        "game_source_scaffold" => scaffold(cwd, config, &job)?,
        "game_build_playtest" => build_playtest(cwd, config, &job)?,
        "game_iteration" => iteration(cwd, config, &job)?,
        _ => {
            return Err(McpError::InvalidRequest(
                "unsupported local game workflow".into(),
            ))
        }
    };
    let asset_id = result
        .get("asset_id")
        .and_then(Value::as_str)
        .map(str::to_owned);
    let hard_fail = result
        .get("hard_fail")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    job.status = if hard_fail {
        CreativeJobStatus::Failed
    } else {
        CreativeJobStatus::Completed
    };
    job.failure_code = hard_fail.then(|| "game_playtest_hard_fail".into());
    job.output_asset_ids = asset_id.clone().into_iter().collect();
    job.actual_output_bytes = asset_id.as_deref().and_then(|id| {
        store::load_project(cwd, config, &job.project_id)
            .ok()
            .and_then(|project| project.asset(id).map(|asset| asset.bytes))
    });
    job.node_runs = vec![NodeRunRecord {
        node_id: workflow_id.into(),
        status: job.status.clone(),
        output: Some(result),
        failure_code: job.failure_code.clone(),
        reused: false,
        execution_batch: 0,
    }];
    job.updated_at_ms = store::now_ms();
    Ok(job)
}

fn scaffold(
    cwd: Option<&str>,
    config: &ServerConfig,
    job: &CreativeJobRecord,
) -> Result<Value, McpError> {
    let game_id = required_str(&job.execution_parameters, "game_id")?;
    let template = required_str(&job.execution_parameters, "template_family")?;
    let mut project = store::load_project(cwd, config, &job.project_id)?;
    let game = project
        .games
        .iter()
        .find(|game| game.game_id == game_id)
        .cloned()
        .ok_or_else(|| McpError::InvalidRequest("unknown game manifest".into()))?;
    let source_revision = format!("source_{}", short_job_id(&job.job_id));
    let root = format!("creative/{}/games/{game_id}/source", job.project_id);
    let index = reviewed_index_html(&game, template);
    let script = reviewed_game_js(&game, template)?;
    let css = reviewed_css();
    write(cwd, config, &format!("{root}/index.html"), index.as_bytes())?;
    write(cwd, config, &format!("{root}/game.js"), script.as_bytes())?;
    write(cwd, config, &format!("{root}/style.css"), css.as_bytes())?;
    let manifest_bytes = serde_json::to_vec_pretty(&game)
        .map_err(|_| McpError::Internal("game manifest encoding failed".into()))?;
    let manifest_path = format!("{root}/game-manifest.json");
    write(cwd, config, &manifest_path, &manifest_bytes)?;
    let (_project, asset_id) = store::register_asset(
        cwd,
        config,
        &job.project_id,
        AssetRegistrationInput {
            path: manifest_path,
            media_type: "application/json".into(),
            role: "game_source_manifest".into(),
            source: AssetSource::GeneratedAsset,
            source_surface: AssetSurface::Game,
            state: AssetState::Candidate,
            job_id: Some(job.job_id.clone()),
            parent_asset_id: None,
            element_id: None,
            dependency_element_ids: game.style_element_id.clone().into_iter().collect(),
            metadata: AssetMetadata {
                artifact_kind: Some("game_source_scaffold".into()),
                artifact_version: Some(source_revision.clone()),
                ..AssetMetadata::default()
            },
        },
    )?;
    if let Some(stored) = project
        .games
        .iter_mut()
        .find(|value| value.game_id == game_id)
    {
        stored.build_state.source_revision = Some(source_revision.clone());
        stored.build_state.build_revision = None;
        stored.build_state.accepted_build_revision = None;
    }
    let game = project
        .games
        .into_iter()
        .find(|value| value.game_id == game_id)
        .expect("game exists");
    store::upsert_game(cwd, config, &job.project_id, game)?;
    Ok(json!({
        "asset_id":asset_id,
        "game_id":game_id,
        "source_revision":source_revision,
        "template_family":template,
        "source_root":root,
        "editable_source":true,
        "build_performed":false,
        "deploy_performed":false,
        "publish_performed":false
    }))
}

fn build_playtest(
    cwd: Option<&str>,
    config: &ServerConfig,
    job: &CreativeJobRecord,
) -> Result<Value, McpError> {
    let game_id = required_str(&job.execution_parameters, "game_id")?;
    let project = store::load_project(cwd, config, &job.project_id)?;
    let mut game = project
        .games
        .iter()
        .find(|game| game.game_id == game_id)
        .cloned()
        .ok_or_else(|| McpError::InvalidRequest("unknown game manifest".into()))?;
    let source_revision = game.build_state.source_revision.clone().ok_or_else(|| {
        McpError::InvalidRequest("game source scaffold has not been created".into())
    })?;
    if let Some(requested) = job
        .execution_parameters
        .get("source_revision")
        .and_then(Value::as_str)
    {
        if requested != source_revision {
            return Err(McpError::InvalidRequest(
                "game build source revision does not match current source state".into(),
            ));
        }
    }
    let source_root = format!("creative/{}/games/{game_id}/source", job.project_id);
    let build_root = format!("creative/{}/games/{game_id}/build", job.project_id);
    for file in ["index.html", "game.js", "style.css", "game-manifest.json"] {
        let execution_root = config
            .resolved_execution_root()
            .map_err(|_| McpError::InvalidRequest("execution root is unavailable".into()))?;
        let source = crate::core::workspace_path::resolve_existing_path(
            &execution_root,
            cwd,
            &format!("{source_root}/{file}"),
            crate::core::workspace_path::EntryKind::File,
        )?;
        let bytes = fs::read(source)
            .map_err(|_| McpError::InvalidRequest("game source file cannot be read".into()))?;
        write(cwd, config, &format!("{build_root}/{file}"), &bytes)?;
    }
    let mut missing_roles = Vec::new();
    let mut materialized_roles = Vec::new();
    for role in &game.asset_roles {
        let Some(asset_id) = role.asset_id.as_deref() else {
            missing_roles.push(role.role.clone());
            continue;
        };
        let asset = project
            .asset(asset_id)
            .ok_or_else(|| McpError::InvalidRequest("game role references unknown asset".into()))?;
        if asset.state != AssetState::Accepted {
            missing_roles.push(role.role.clone());
            continue;
        }
        let source = store::resolve_registered_asset_path(cwd, config, &job.project_id, asset_id)?;
        let bytes = fs::read(source)
            .map_err(|_| McpError::InvalidRequest("game asset cannot be materialized".into()))?;
        let target = format!("{build_root}/assets/{}", role.runtime_path);
        write(cwd, config, &target, &bytes)?;
        materialized_roles.push(role.role.clone());
    }
    let supported_inputs = ["keyboard", "mouse", "touch", "gamepad"];
    let unsupported_inputs = game
        .inputs
        .iter()
        .filter(|input| !supported_inputs.contains(&input.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    let hard_fail = !missing_roles.is_empty() || !unsupported_inputs.is_empty();
    let build_revision = format!("build_{}", short_job_id(&job.job_id));
    let evidence = json!({
        "schema":"game-playtest-evidence-v1",
        "game_id":game_id,
        "source_revision":source_revision,
        "build_revision":build_revision,
        "structural_checks":{
            "start_defined":true,
            "core_loop_defined":!game.core_loop.is_empty(),
            "win_condition_defined":!game.win_condition.is_empty(),
            "lose_condition_defined":!game.lose_condition.is_empty(),
            "restart_defined":!game.restart_behavior.is_empty(),
            "missing_asset_roles":missing_roles,
            "unsupported_inputs":unsupported_inputs,
            "materialized_asset_roles":materialized_roles,
            "target_fps":game.runtime_budget.target_fps
        },
        "browser_runtime_inspection":"not_inspected",
        "console_error_inspection":"not_inspected",
        "responsive_render_inspection":"not_inspected",
        "timing_runtime_inspection":"not_inspected",
        "hard_fail":hard_fail
    });
    let evidence_bytes = serde_json::to_vec_pretty(&evidence)
        .map_err(|_| McpError::Internal("game playtest evidence encoding failed".into()))?;
    let evidence_path = format!("{build_root}/playtest-{build_revision}.json");
    write(cwd, config, &evidence_path, &evidence_bytes)?;
    let (_project, asset_id) = store::register_asset(
        cwd,
        config,
        &job.project_id,
        AssetRegistrationInput {
            path: evidence_path,
            media_type: "application/json".into(),
            role: "game_playtest_evidence".into(),
            source: AssetSource::GeneratedAsset,
            source_surface: AssetSurface::Game,
            state: AssetState::Candidate,
            job_id: Some(job.job_id.clone()),
            parent_asset_id: None,
            element_id: None,
            dependency_element_ids: game.style_element_id.clone().into_iter().collect(),
            metadata: AssetMetadata {
                artifact_kind: Some("game_build_playtest".into()),
                artifact_version: Some(build_revision.clone()),
                ..AssetMetadata::default()
            },
        },
    )?;
    let finding = QaFinding {
        finding_id: format!("finding_{}", Uuid::new_v4().simple()),
        domain: "game_playtest".into(),
        severity: if hard_fail {
            QaSeverity::HardFail
        } else {
            QaSeverity::NotInspected
        },
        subject_id: game_id.to_owned(),
        message: if hard_fail {
            "Deterministic game build/playtest preflight found blocking issues.".into()
        } else {
            "Deterministic build preflight passed; browser runtime behavior still requires E2E inspection.".into()
        },
        source_revision_id: Some(build_revision.clone()),
        asset_id: Some(asset_id.clone()),
        evaluator_binding_id: None,
        evidence: evidence.clone(),
        created_at_ms: store::now_ms(),
    };
    store::add_qa_finding(cwd, config, &job.project_id, finding)?;
    game.build_state.build_revision = Some(build_revision.clone());
    game.build_state.accepted_build_revision = (!hard_fail).then_some(build_revision.clone());
    store::upsert_game(cwd, config, &job.project_id, game)?;
    Ok(json!({
        "asset_id":asset_id,
        "game_id":game_id,
        "build_revision":build_revision,
        "hard_fail":hard_fail,
        "evidence":evidence,
        "deploy_performed":false,
        "publish_performed":false
    }))
}

fn iteration(
    cwd: Option<&str>,
    config: &ServerConfig,
    job: &CreativeJobRecord,
) -> Result<Value, McpError> {
    let game_id = required_str(&job.execution_parameters, "game_id")?;
    let gameplay_impacting = job
        .execution_parameters
        .get("gameplay_impacting")
        .and_then(Value::as_bool)
        .ok_or_else(|| McpError::InvalidRequest("gameplay_impacting is required".into()))?;
    let changed_fields = job
        .execution_parameters
        .get("changed_fields")
        .and_then(Value::as_array)
        .ok_or_else(|| McpError::InvalidRequest("changed_fields are required".into()))?;
    let project = store::load_project(cwd, config, &job.project_id)?;
    let mut game = project
        .games
        .iter()
        .find(|game| game.game_id == game_id)
        .cloned()
        .ok_or_else(|| McpError::InvalidRequest("unknown game manifest".into()))?;
    let previous_build = game.build_state.accepted_build_revision.clone();
    if gameplay_impacting {
        game.build_state.build_revision = None;
        game.build_state.accepted_build_revision = None;
    }
    store::upsert_game(cwd, config, &job.project_id, game)?;
    let receipt = json!({
        "schema":"game-iteration-v1",
        "game_id":game_id,
        "changed_fields":changed_fields,
        "gameplay_impacting":gameplay_impacting,
        "previous_accepted_build_revision":previous_build,
        "build_invalidated":gameplay_impacting,
        "source_preserved":true,
        "unchanged_assets_preserved":true,
        "playtest_required":gameplay_impacting
    });
    let bytes = serde_json::to_vec_pretty(&receipt)
        .map_err(|_| McpError::Internal("game iteration receipt encoding failed".into()))?;
    let path = format!(
        "creative/{}/games/{game_id}/iterations/{}.json",
        job.project_id, job.job_id
    );
    write(cwd, config, &path, &bytes)?;
    let (_project, asset_id) = store::register_asset(
        cwd,
        config,
        &job.project_id,
        AssetRegistrationInput {
            path,
            media_type: "application/json".into(),
            role: "game_iteration_receipt".into(),
            source: AssetSource::GeneratedAsset,
            source_surface: AssetSurface::Game,
            state: AssetState::Candidate,
            job_id: Some(job.job_id.clone()),
            parent_asset_id: None,
            element_id: None,
            dependency_element_ids: Vec::new(),
            metadata: AssetMetadata {
                artifact_kind: Some("game_iteration".into()),
                artifact_version: Some("v1".into()),
                ..AssetMetadata::default()
            },
        },
    )?;
    Ok(json!({"asset_id":asset_id,"hard_fail":false,"receipt":receipt}))
}

fn reviewed_index_html(game: &GameManifest, template: &str) -> String {
    format!(
        "<!doctype html><html><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>{}</title><link rel=\"stylesheet\" href=\"style.css\"></head><body data-template=\"{}\"><main><canvas id=\"game\" width=\"960\" height=\"540\"></canvas><div id=\"status\"></div></main><script type=\"module\" src=\"game.js\"></script></body></html>",
        html_escape(&game.title),
        template
    )
}

fn reviewed_game_js(game: &GameManifest, template: &str) -> Result<String, McpError> {
    let manifest = serde_json::to_string(game)
        .map_err(|_| McpError::Internal("game source manifest encoding failed".into()))?;
    Ok(format!(
        "// Reviewed Masih Awam browser-game scaffold; source stays editable.\nconst manifest={manifest};\nconst template={template:?};\nconst canvas=document.getElementById('game');const ctx=canvas.getContext('2d');const status=document.getElementById('status');\nlet state={{phase:'start',score:0,lives:3,t:0}};\nfunction restart(){{state={{phase:'core',score:0,lives:3,t:0}};}}\nfunction win(){{state.phase='win';}} function lose(){{state.phase='lose';}}\nfunction update(dt){{if(state.phase==='core'){{state.t+=dt;if(state.t>30)win();}}}}\nfunction render(){{ctx.clearRect(0,0,canvas.width,canvas.height);ctx.fillText(manifest.title,24,36);ctx.fillText('phase: '+state.phase,24,64);status.textContent=manifest.core_loop;}}\nlet last=performance.now();function frame(now){{const dt=Math.min((now-last)/1000,0.05);last=now;update(dt);render();requestAnimationFrame(frame);}}\naddEventListener('keydown',e=>{{if(e.key==='r'||e.key==='Enter')restart();}});restart();requestAnimationFrame(frame);\nexport{{manifest,state,restart,win,lose,update,render,template}};\n"
    ))
}

fn reviewed_css() -> &'static str {
    "html,body{margin:0;min-height:100%;background:#111;color:#eee;font-family:system-ui}main{display:grid;place-items:center;gap:1rem;padding:1rem}canvas{max-width:100%;height:auto;background:#20242a;border:1px solid #555}"
}

fn html_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn write(
    cwd: Option<&str>,
    config: &ServerConfig,
    path: &str,
    bytes: &[u8],
) -> Result<(), McpError> {
    crate::application::workspace::write_contained_bytes(path, cwd, bytes, true, true, config)
}

fn required_str<'a>(value: &'a Value, field: &str) -> Result<&'a str, McpError> {
    value
        .get(field)
        .and_then(Value::as_str)
        .filter(|value| {
            !value.is_empty() && value.len() <= 128 && !value.chars().any(char::is_control)
        })
        .ok_or_else(|| McpError::InvalidRequest(format!("game {field} is required")))
}

fn short_job_id(job_id: &str) -> &str {
    job_id.strip_prefix("job_").unwrap_or(job_id)
}
