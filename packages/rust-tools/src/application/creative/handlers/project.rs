use super::*;

pub(in crate::application::creative) fn project(
    arguments: &Value,
    config: &ServerConfig,
    owner: &str,
) -> Result<ToolCallResult, McpError> {
    let action = required_str(arguments, "action")?;
    let cwd = arguments.get("cwd").and_then(Value::as_str);
    match action {
        "create" => {
            let tracks: Vec<CreativeTrack> = parse_required(arguments, "tracks")?;
            let target = parse_optional(arguments, "target")?;
            let created = store::create_project(
                cwd,
                config,
                store::NewProject {
                    project_id: optional_string(arguments, "project_id"),
                    title: required_str(arguments, "title")?.to_owned(),
                    intent: required_str(arguments, "intent")?.to_owned(),
                    tracks,
                    target,
                },
            )?;
            complete(json!({
                "layout": store::project_layout(&created.project_id)?,
                "project": created
            }))
        }
        "get" => {
            let project = store::load_project(cwd, config, required_str(arguments, "project_id")?)?;
            complete(json!({
                "layout": store::project_layout(&project.project_id)?,
                "project": project
            }))
        }
        "list" => complete(json!({"project_ids": store::list_projects(cwd, config)?})),
        "scene_board_put" => {
            let project_id = required_str(arguments, "project_id")?;
            let board: SceneBoard = parse_required(arguments, "scene_board")?;
            let project = store::upsert_scene_board(cwd, config, project_id, board)?;
            complete(
                json!({"scene_boards": project.scene_boards, "project_updated_at_ms": project.updated_at_ms}),
            )
        }
        "scene_board_get" => {
            let project = store::load_project(cwd, config, required_str(arguments, "project_id")?)?;
            let board_id = required_str(arguments, "board_id")?;
            let board = project
                .scene_boards
                .iter()
                .find(|value| value.board_id == board_id)
                .ok_or_else(|| McpError::InvalidRequest("unknown creative scene board".into()))?;
            complete(json!({"scene_board": board}))
        }
        "scene_put" => {
            let project_id = required_str(arguments, "project_id")?;
            let scene: SceneManifest = parse_required(arguments, "scene")?;
            let project = store::upsert_scene(cwd, config, project_id, scene)?;
            complete(
                json!({"scenes": project.scenes, "project_updated_at_ms": project.updated_at_ms}),
            )
        }
        "scene_get" => {
            let project = store::load_project(cwd, config, required_str(arguments, "project_id")?)?;
            let scene_id = required_str(arguments, "scene_id")?;
            let scene = project
                .scenes
                .iter()
                .find(|value| value.scene_id == scene_id)
                .ok_or_else(|| McpError::InvalidRequest("unknown creative scene".into()))?;
            complete(json!({"scene": scene}))
        }
        "game_put" => {
            let project_id = required_str(arguments, "project_id")?;
            let game: GameManifest = parse_required(arguments, "game")?;
            let project = store::upsert_game(cwd, config, project_id, game)?;
            complete(
                json!({"games": project.games, "project_updated_at_ms": project.updated_at_ms}),
            )
        }
        "game_get" => {
            let project = store::load_project(cwd, config, required_str(arguments, "project_id")?)?;
            let game_id = required_str(arguments, "game_id")?;
            let game = project
                .games
                .iter()
                .find(|value| value.game_id == game_id)
                .ok_or_else(|| McpError::InvalidRequest("unknown creative game".into()))?;
            complete(json!({"game": game}))
        }
        "audio_put" => {
            let project_id = required_str(arguments, "project_id")?;
            let audio_plan: AudioPlan = parse_required(arguments, "audio_plan")?;
            let project = store::upsert_audio_plan(cwd, config, project_id, audio_plan)?;
            complete(
                json!({"audio_plans": project.audio_plans, "project_updated_at_ms": project.updated_at_ms}),
            )
        }
        "audio_get" => {
            let project = store::load_project(cwd, config, required_str(arguments, "project_id")?)?;
            let audio_plan_id = required_str(arguments, "audio_plan_id")?;
            let audio_plan = project
                .audio_plans
                .iter()
                .find(|value| value.audio_plan_id == audio_plan_id)
                .ok_or_else(|| McpError::InvalidRequest("unknown creative audio plan".into()))?;
            complete(json!({"audio_plan": audio_plan}))
        }
        "qa_add" => {
            let project_id = required_str(arguments, "project_id")?;
            let severity: QaSeverity = parse_required(arguments, "severity")?;
            let finding = QaFinding {
                finding_id: format!("finding_{}", uuid::Uuid::new_v4().simple()),
                domain: required_str(arguments, "domain")?.to_owned(),
                severity,
                subject_id: required_str(arguments, "subject_id")?.to_owned(),
                message: required_str(arguments, "message")?.to_owned(),
                source_revision_id: optional_string(arguments, "source_revision_id"),
                asset_id: optional_string(arguments, "asset_id"),
                evaluator_binding_id: optional_string(arguments, "evaluator_binding_id"),
                evidence: arguments
                    .get("evidence")
                    .cloned()
                    .unwrap_or_else(|| json!({})),
                created_at_ms: store::now_ms(),
            };
            let project = store::add_qa_finding(cwd, config, project_id, finding.clone())?;
            complete(json!({"finding": finding, "project_updated_at_ms": project.updated_at_ms}))
        }
        "qa_list" => {
            let project = store::load_project(cwd, config, required_str(arguments, "project_id")?)?;
            complete(json!({"qa_findings": project.qa_findings}))
        }
        "room_create" => {
            let project_id = required_str(arguments, "project_id")?;
            let game_id = required_str(arguments, "game_id")?;
            let member_id = required_str(arguments, "member_id")?;
            validate_id(member_id, "multiplayer member id")?;
            let project = store::load_project(cwd, config, project_id)?;
            let game = project
                .games
                .iter()
                .find(|game| game.game_id == game_id)
                .ok_or_else(|| McpError::InvalidRequest("unknown creative game".into()))?;
            if game.player_mode != PlayerMode::OnlineMultiplayer {
                return Err(McpError::InvalidRequest(
                    "multiplayer rooms require an online_multiplayer Game Manifest".into(),
                ));
            }
            let now = store::now_ms();
            let room = MultiplayerRoomState {
                room_id: format!("room_{}", uuid::Uuid::new_v4().simple()),
                project_id: project_id.to_owned(),
                game_id: game_id.to_owned(),
                owner: owner.to_owned(),
                member_ids: vec![member_id.to_owned()],
                shared_state: arguments
                    .get("shared_state")
                    .cloned()
                    .unwrap_or_else(|| json!({})),
                revision: 1,
                closed: false,
                created_at_ms: now,
                updated_at_ms: now,
            };
            store::store_multiplayer_room(cwd, config, &room)?;
            complete(json!({"room": room}))
        }
        "room_join" | "room_update" | "room_get" | "room_leave" => {
            let project_id = required_str(arguments, "project_id")?;
            let game_id = required_str(arguments, "game_id")?;
            let room_id = required_str(arguments, "room_id")?;
            let mut room = store::load_multiplayer_room(cwd, config, project_id, game_id, room_id)?;
            if room.owner != owner {
                return Err(McpError::InvalidRequest(
                    "multiplayer room is not owned by the authenticated caller".into(),
                ));
            }
            match action {
                "room_get" => complete(json!({"room": room})),
                "room_join" => {
                    if room.closed {
                        return Err(McpError::InvalidRequest(
                            "multiplayer room is closed".into(),
                        ));
                    }
                    let member_id = required_str(arguments, "member_id")?;
                    validate_id(member_id, "multiplayer member id")?;
                    if !room.member_ids.iter().any(|value| value == member_id) {
                        if room.member_ids.len() >= 16 {
                            return Err(McpError::InvalidRequest(
                                "multiplayer room is full".into(),
                            ));
                        }
                        room.member_ids.push(member_id.to_owned());
                    }
                    room.revision = room.revision.saturating_add(1);
                    room.updated_at_ms = store::now_ms();
                    store::store_multiplayer_room(cwd, config, &room)?;
                    complete(json!({"room": room}))
                }
                "room_update" => {
                    if room.closed {
                        return Err(McpError::InvalidRequest(
                            "multiplayer room is closed".into(),
                        ));
                    }
                    let expected_revision = arguments
                        .get("expected_revision")
                        .and_then(Value::as_u64)
                        .ok_or_else(|| {
                            McpError::InvalidRequest("expected_revision is required".into())
                        })?;
                    if expected_revision != room.revision {
                        return Err(McpError::InvalidRequest(
                            "multiplayer room revision conflict".into(),
                        ));
                    }
                    let shared_state = arguments.get("shared_state").cloned().ok_or_else(|| {
                        McpError::InvalidRequest("shared_state is required".into())
                    })?;
                    validate_spec(&shared_state)?;
                    room.shared_state = shared_state;
                    room.revision = room.revision.saturating_add(1);
                    room.updated_at_ms = store::now_ms();
                    store::store_multiplayer_room(cwd, config, &room)?;
                    complete(json!({"room": room}))
                }
                "room_leave" => {
                    let member_id = required_str(arguments, "member_id")?;
                    room.member_ids.retain(|value| value != member_id);
                    room.closed = room.member_ids.is_empty();
                    room.revision = room.revision.saturating_add(1);
                    room.updated_at_ms = store::now_ms();
                    store::store_multiplayer_room(cwd, config, &room)?;
                    complete(json!({"room": room}))
                }
                _ => unreachable!(),
            }
        }
        _ => Err(McpError::InvalidRequest(
            "unsupported creative project action".into(),
        )),
    }
}
