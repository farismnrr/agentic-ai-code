use super::*;

pub(in crate::application::creative) struct PartialGraphExecution<'a> {
    pub previous: &'a CreativeJobRecord,
    pub changed_node_ids: &'a [String],
    pub approved: bool,
}

pub(in crate::application::creative) async fn execute_graph_direct(
    cwd: Option<&str>,
    config: &ServerConfig,
    owner: &str,
    graph: &graph::CreativeGraph,
    project: &crate::application::creative::contracts::CreativeProject,
    approved: bool,
) -> Result<CreativeJobRecord, McpError> {
    let mut parent = new_graph_parent_job(project, owner, &graph.graph_id, approved);
    store::store_job(cwd, config, &parent)?;
    let external = graph_external_executor(cwd, config, owner, &parent);
    let executed = graph::execute_graph(
        graph,
        project,
        config,
        GraphExecutionContext {
            owner,
            now_ms: store::now_ms(),
            job_id: &parent.job_id,
            external,
        },
    )
    .await?;
    parent.status = executed.status;
    parent.node_runs = executed.node_runs;
    parent.output_asset_ids = executed.output_asset_ids;
    parent.actual_output_bytes = executed.actual_output_bytes;
    parent.failure_code = executed.failure_code;
    parent.updated_at_ms = store::now_ms();
    store::store_job(cwd, config, &parent)?;
    Ok(parent)
}

pub(in crate::application::creative) async fn execute_graph_partial_direct(
    cwd: Option<&str>,
    config: &ServerConfig,
    owner: &str,
    graph: &graph::CreativeGraph,
    project: &crate::application::creative::contracts::CreativeProject,
    partial: PartialGraphExecution<'_>,
) -> Result<CreativeJobRecord, McpError> {
    let mut parent = new_graph_parent_job(project, owner, &graph.graph_id, partial.approved);
    store::store_job(cwd, config, &parent)?;
    let external = graph_external_executor(cwd, config, owner, &parent);
    let executed = graph::execute_graph_partial(
        graph,
        project,
        config,
        partial.previous,
        partial.changed_node_ids,
        GraphExecutionContext {
            owner,
            now_ms: store::now_ms(),
            job_id: &parent.job_id,
            external,
        },
    )
    .await?;
    parent.status = executed.status;
    parent.node_runs = executed.node_runs;
    parent.output_asset_ids = executed.output_asset_ids;
    parent.actual_output_bytes = executed.actual_output_bytes;
    parent.failure_code = executed.failure_code;
    parent.updated_at_ms = store::now_ms();
    store::store_job(cwd, config, &parent)?;
    Ok(parent)
}

fn new_graph_parent_job(
    project: &crate::application::creative::contracts::CreativeProject,
    owner: &str,
    graph_id: &str,
    approved: bool,
) -> CreativeJobRecord {
    let now = store::now_ms();
    CreativeJobRecord {
        schema_version: CREATIVE_SCHEMA_VERSION,
        job_id: format!("job_{}", Uuid::new_v4().simple()),
        project_id: project.project_id.clone(),
        owner: owner.to_owned(),
        kind: CreativeJobKind::Graph,
        graph_id: Some(graph_id.to_owned()),
        capability_id: None,
        workflow_id: None,
        execution_binding_id: None,
        execution_parameters: Value::Object(Default::default()),
        compiler_version: None,
        execution_binding_version: None,
        changed_fields: Vec::new(),
        status: CreativeJobStatus::Running,
        estimate: None,
        approved,
        retry_count: 0,
        max_retries: 0,
        timeout_ms: 0,
        created_at_ms: now,
        updated_at_ms: now,
        node_runs: Vec::new(),
        output_asset_ids: Vec::new(),
        actual_output_bytes: None,
        failure_code: None,
    }
}

pub(in crate::application::creative) async fn execute_graph_job(
    cwd: Option<&str>,
    config: &ServerConfig,
    owner: &str,
    queued: &CreativeJobRecord,
) -> Result<CreativeJobRecord, McpError> {
    let graph_id = queued
        .graph_id
        .as_deref()
        .ok_or_else(|| McpError::InvalidRequest("graph creative job requires graph_id".into()))?;
    let graph = store::load_graph(cwd, config, &queued.project_id, graph_id)?;
    let project = store::load_project(cwd, config, &queued.project_id)?;
    let external = graph_external_executor(cwd, config, owner, queued);
    let executed = graph::execute_graph(
        &graph,
        &project,
        config,
        GraphExecutionContext {
            owner,
            now_ms: store::now_ms(),
            job_id: &queued.job_id,
            external,
        },
    )
    .await?;
    let mut terminal = queued.clone();
    terminal.status = executed.status;
    terminal.node_runs = executed.node_runs;
    terminal.output_asset_ids = executed.output_asset_ids;
    terminal.actual_output_bytes = executed.actual_output_bytes;
    terminal.failure_code = executed.failure_code;
    terminal.updated_at_ms = store::now_ms();
    Ok(terminal)
}

fn graph_external_executor(
    cwd: Option<&str>,
    config: &ServerConfig,
    owner: &str,
    parent: &CreativeJobRecord,
) -> ExternalNodeExecutor {
    let cwd = cwd.map(str::to_owned);
    let config = config.clone();
    let owner = owner.to_owned();
    let parent = parent.clone();
    Arc::new(move |node: GraphNode, upstream: Vec<Value>| {
        let cwd = cwd.clone();
        let config = config.clone();
        let owner = owner.clone();
        let parent = parent.clone();
        Box::pin(async move {
            execute_external_graph_node(cwd.as_deref(), &config, &owner, &parent, node, upstream)
                .await
        })
    })
}

async fn execute_external_graph_node(
    cwd: Option<&str>,
    config: &ServerConfig,
    owner: &str,
    parent: &CreativeJobRecord,
    node: GraphNode,
    upstream: Vec<Value>,
) -> Result<Value, String> {
    match node.kind {
        GraphNodeKind::StoryboardStore => {
            let scene_value = node
                .inputs
                .get("scene")
                .cloned()
                .unwrap_or_else(|| node.inputs.clone());
            let scene: SceneManifest = serde_json::from_value(scene_value)
                .map_err(|_| "scene_manifest_invalid".to_owned())?;
            let project = store::upsert_scene(cwd, config, &parent.project_id, scene.clone())
                .map_err(|_| "scene_store_failed".to_owned())?;
            Ok(json!({
                "scene_id":scene.scene_id,
                "stored":true,
                "project_updated_at_ms":project.updated_at_ms,
                "upstream":upstream
            }))
        }
        GraphNodeKind::SceneManifestValidate => {
            let scene_value = node
                .inputs
                .get("scene")
                .cloned()
                .unwrap_or_else(|| node.inputs.clone());
            let scene: SceneManifest = serde_json::from_value(scene_value)
                .map_err(|_| "scene_manifest_invalid".to_owned())?;
            let mut project = store::load_project(cwd, config, &parent.project_id)
                .map_err(|_| "scene_project_unavailable".to_owned())?;
            if let Some(existing) = project
                .scenes
                .iter_mut()
                .find(|value| value.scene_id == scene.scene_id)
            {
                *existing = scene.clone();
            } else {
                project.scenes.push(scene.clone());
            }
            project
                .validate()
                .map_err(|_| "scene_manifest_invalid".to_owned())?;
            Ok(json!({
                "scene_id":scene.scene_id,
                "valid":true,
                "stored":false,
                "upstream":upstream
            }))
        }
        GraphNodeKind::DccExecute => {
            if !parent.approved {
                return Err("approval_required".into());
            }
            execute_dcc_graph_node(cwd, config, owner, parent, node).await
        }
        GraphNodeKind::DccRender => {
            if !parent.approved {
                return Err("approval_required".into());
            }
            blender_graph_call(
                cwd,
                config,
                owner,
                &parent.project_id,
                "blender_render",
                node.inputs,
            )
            .await
        }
        GraphNodeKind::GameBuild | GraphNodeKind::GamePlaytest => {
            let mut child = graph_child_job(parent, &node);
            child.kind = CreativeJobKind::Workflow;
            child.workflow_id = Some("game_build_playtest".into());
            child.capability_id = None;
            child.execution_binding_id = None;
            let executed = execute_bound_job(cwd, config, owner, child)
                .await
                .map_err(|_| "game_build_execution_failed".to_owned())?;
            graph_child_output(executed)
        }
        GraphNodeKind::GameDeploy => {
            execute_graph_capability(cwd, config, parent, node, "game.deploy").await
        }
        GraphNodeKind::AssembleSequence => {
            let mut child = graph_child_job(parent, &node);
            child.kind = CreativeJobKind::Workflow;
            child.workflow_id = Some("sequence_assemble".into());
            child.capability_id = None;
            let executed = execute_bound_job(cwd, config, owner, child)
                .await
                .map_err(|_| "sequence_assembly_failed".to_owned())?;
            graph_child_output(executed)
        }
        GraphNodeKind::ExportArtifact => {
            let mut child = graph_child_job(parent, &node);
            child.kind = CreativeJobKind::Workflow;
            child.workflow_id = Some("export_profile".into());
            child.capability_id = None;
            child.execution_binding_id = None;
            let executed = execute_bound_job(cwd, config, owner, child)
                .await
                .map_err(|_| "export_execution_failed".to_owned())?;
            graph_child_output(executed)
        }
        GraphNodeKind::GenerateImage | GraphNodeKind::GenerateVideo | GraphNodeKind::Generate3d => {
            let capability_id = graph::capability_for_node(&node.kind)
                .ok_or_else(|| "graph_capability_missing".to_owned())?;
            execute_graph_capability(cwd, config, parent, node, capability_id).await
        }
        _ => Err("graph_external_node_unsupported".into()),
    }
}

async fn execute_graph_capability(
    cwd: Option<&str>,
    config: &ServerConfig,
    parent: &CreativeJobRecord,
    node: GraphNode,
    capability_id: &str,
) -> Result<Value, String> {
    let descriptor = registry::validate_capability_parameters(capability_id, &node.inputs)
        .map_err(|_| "graph_capability_parameters_invalid".to_owned())?;
    if descriptor.requires_execution_binding {
        registry::validate_binding_selection(
            config,
            capability_id,
            node.execution_binding_id.as_deref(),
        )
        .map_err(|_| "graph_execution_binding_invalid".to_owned())?;
    }
    let mut child = graph_child_job(parent, &node);
    child.kind = CreativeJobKind::Capability;
    child.capability_id = Some(capability_id.to_owned());
    child.workflow_id = None;
    let executed = execute_leaf_bound_job(cwd, config, child)
        .map_err(|_| "graph_capability_execution_failed".to_owned())?;
    graph_child_output(executed)
}

async fn execute_dcc_graph_node(
    cwd: Option<&str>,
    config: &ServerConfig,
    owner: &str,
    parent: &CreativeJobRecord,
    node: GraphNode,
) -> Result<Value, String> {
    let operation = node
        .inputs
        .get("operation")
        .and_then(Value::as_str)
        .ok_or_else(|| "dcc_operation_required".to_owned())?
        .to_owned();
    if matches!(
        operation.as_str(),
        "character_mesh_production"
            | "character_rig_production"
            | "character_action"
            | "character_secondary_motion"
    ) {
        let mut child = graph_child_job(parent, &node);
        if let Some(object) = child.execution_parameters.as_object_mut() {
            object.remove("operation");
        }
        child.kind = CreativeJobKind::Workflow;
        child.workflow_id = Some(operation);
        child.capability_id = None;
        child.execution_binding_id = None;
        let executed = execute_bound_job(cwd, config, owner, child)
            .await
            .map_err(|_| "dcc_character_workflow_failed".to_owned())?;
        return graph_child_output(executed);
    }
    let tool = match operation.as_str() {
        "asset_import" => "blender_asset_import",
        "asset_export" => "blender_asset_export",
        "checkpoint_create" => "blender_checkpoint_create",
        "checkpoint_restore" => "blender_checkpoint_restore",
        _ => return Err("dcc_operation_unsupported".into()),
    };
    let mut arguments = node.inputs;
    if let Some(object) = arguments.as_object_mut() {
        object.remove("operation");
    }
    blender_graph_call(cwd, config, owner, &parent.project_id, tool, arguments).await
}

async fn blender_graph_call(
    cwd: Option<&str>,
    config: &ServerConfig,
    owner: &str,
    project_id: &str,
    tool: &str,
    mut arguments: Value,
) -> Result<Value, String> {
    let object = arguments
        .as_object_mut()
        .ok_or_else(|| "blender_graph_arguments_invalid".to_owned())?;
    object.insert("project_id".into(), Value::String(project_id.to_owned()));
    if let Some(cwd) = cwd {
        object.insert("cwd".into(), Value::String(cwd.to_owned()));
    }
    let result = crate::application::blender::dispatch_tool(tool, &arguments, config, owner)
        .await
        .map_err(|_| "blender_graph_execution_failed".to_owned())?
        .ok_or_else(|| "blender_graph_tool_unavailable".to_owned())?;
    if result.is_error {
        return Err("blender_graph_execution_failed".into());
    }
    result
        .content
        .first()
        .and_then(|content| serde_json::from_str::<Value>(&content.text).ok())
        .ok_or_else(|| "blender_graph_result_invalid".to_owned())
}

fn graph_child_job(parent: &CreativeJobRecord, node: &GraphNode) -> CreativeJobRecord {
    let mut child = parent.clone();
    child.kind = CreativeJobKind::Capability;
    child.graph_id = None;
    child.capability_id = None;
    child.workflow_id = None;
    child.execution_binding_id = node.execution_binding_id.clone();
    child.execution_parameters = node.inputs.clone();
    child.compiler_version = None;
    child.execution_binding_version = None;
    child.changed_fields.clear();
    child.status = CreativeJobStatus::Running;
    child.node_runs.clear();
    child.output_asset_ids.clear();
    child.actual_output_bytes = None;
    child.failure_code = None;
    child.updated_at_ms = store::now_ms();
    child
}

fn graph_child_output(job: CreativeJobRecord) -> Result<Value, String> {
    if job.status != CreativeJobStatus::Completed {
        return Err(job
            .failure_code
            .unwrap_or_else(|| "graph_child_execution_failed".into()));
    }
    Ok(json!({
        "job_id":job.job_id,
        "capability_id":job.capability_id,
        "workflow_id":job.workflow_id,
        "asset_id":job.output_asset_ids.first(),
        "output_asset_ids":job.output_asset_ids,
        "actual_output_bytes":job.actual_output_bytes,
        "node_evidence":job.node_runs
    }))
}
