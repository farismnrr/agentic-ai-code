use super::*;

pub fn store_multiplayer_room(
    cwd: Option<&str>,
    config: &ServerConfig,
    room: &MultiplayerRoomState,
) -> Result<(), McpError> {
    validate_id(&room.project_id, "project_id")?;
    validate_id(&room.game_id, "game_id")?;
    validate_id(&room.room_id, "room_id")?;
    if room.owner.is_empty() || room.owner.len() > 256 {
        return Err(McpError::InvalidRequest(
            "multiplayer room owner is invalid".into(),
        ));
    }
    if room.member_ids.len() > 16 || (!room.closed && room.member_ids.is_empty()) {
        return Err(McpError::InvalidRequest(
            "multiplayer room member count exceeds allowed bounds".into(),
        ));
    }
    for member_id in &room.member_ids {
        validate_id(member_id, "multiplayer member id")?;
    }
    validate_spec(&room.shared_state)?;
    let path = format!(
        "{STATE_PREFIX}/projects/{}/games/{}/rooms/{}.json",
        room.project_id, room.game_id, room.room_id
    );
    write_json(cwd, config, &path, room, true)
}

pub fn load_multiplayer_room(
    cwd: Option<&str>,
    config: &ServerConfig,
    project_id: &str,
    game_id: &str,
    room_id: &str,
) -> Result<MultiplayerRoomState, McpError> {
    validate_id(project_id, "project_id")?;
    validate_id(game_id, "game_id")?;
    validate_id(room_id, "room_id")?;
    let path = format!("{STATE_PREFIX}/projects/{project_id}/games/{game_id}/rooms/{room_id}.json");
    read_json(cwd, config, &path)
}

pub fn project_layout(project_id: &str) -> Result<CreativeProjectLayout, McpError> {
    validate_id(project_id, "project_id")?;
    let state_root = format!("{STATE_PREFIX}/projects/{project_id}");
    let production_root = format!("creative/{project_id}");
    Ok(CreativeProjectLayout {
        state_root: state_root.clone(),
        production_root: production_root.clone(),
        assets_root: format!("{production_root}/assets"),
        scene_boards_root: format!("{production_root}/scene-boards"),
        graphs_root: format!("{state_root}/graphs"),
        templates_root: format!("{state_root}/templates"),
        games_root: format!("{production_root}/games"),
        qa_root: format!("{state_root}/qa"),
        exports_root: format!("{production_root}/exports"),
    })
}

pub fn upsert_scene_board(
    cwd: Option<&str>,
    config: &ServerConfig,
    project_id: &str,
    board: SceneBoard,
) -> Result<CreativeProject, McpError> {
    let mut project = load_project(cwd, config, project_id)?;
    if let Some(existing) = project
        .scene_boards
        .iter_mut()
        .find(|value| value.board_id == board.board_id)
    {
        *existing = board;
    } else {
        if project.scene_boards.len() >= MAX_PROJECT_SCENE_BOARDS {
            return Err(McpError::InvalidRequest(
                "creative scene-board capacity reached".into(),
            ));
        }
        project.scene_boards.push(board);
    }
    persist_project(cwd, config, project)
}

pub fn upsert_scene(
    cwd: Option<&str>,
    config: &ServerConfig,
    project_id: &str,
    scene: SceneManifest,
) -> Result<CreativeProject, McpError> {
    let mut project = load_project(cwd, config, project_id)?;
    if let Some(existing) = project
        .scenes
        .iter_mut()
        .find(|value| value.scene_id == scene.scene_id)
    {
        *existing = scene;
    } else {
        if project.scenes.len() >= MAX_PROJECT_SCENES {
            return Err(McpError::InvalidRequest(
                "creative scene capacity reached".into(),
            ));
        }
        project.scenes.push(scene);
    }
    persist_project(cwd, config, project)
}

pub fn upsert_game(
    cwd: Option<&str>,
    config: &ServerConfig,
    project_id: &str,
    game: GameManifest,
) -> Result<CreativeProject, McpError> {
    let mut project = load_project(cwd, config, project_id)?;
    if let Some(existing) = project
        .games
        .iter_mut()
        .find(|value| value.game_id == game.game_id)
    {
        *existing = game;
    } else {
        if project.games.len() >= MAX_PROJECT_GAMES {
            return Err(McpError::InvalidRequest(
                "creative game capacity reached".into(),
            ));
        }
        project.games.push(game);
    }
    persist_project(cwd, config, project)
}

pub fn upsert_audio_plan(
    cwd: Option<&str>,
    config: &ServerConfig,
    project_id: &str,
    audio_plan: AudioPlan,
) -> Result<CreativeProject, McpError> {
    let mut project = load_project(cwd, config, project_id)?;
    if let Some(existing) = project
        .audio_plans
        .iter_mut()
        .find(|value| value.audio_plan_id == audio_plan.audio_plan_id)
    {
        *existing = audio_plan;
    } else {
        if project.audio_plans.len() >= MAX_PROJECT_AUDIO_PLANS {
            return Err(McpError::InvalidRequest(
                "creative audio-plan capacity reached".into(),
            ));
        }
        project.audio_plans.push(audio_plan);
    }
    persist_project(cwd, config, project)
}

pub fn add_qa_finding(
    cwd: Option<&str>,
    config: &ServerConfig,
    project_id: &str,
    finding: QaFinding,
) -> Result<CreativeProject, McpError> {
    let mut project = load_project(cwd, config, project_id)?;
    if project
        .qa_findings
        .iter()
        .any(|value| value.finding_id == finding.finding_id)
    {
        return Err(McpError::InvalidRequest(
            "creative QA finding identity already exists".into(),
        ));
    }
    if project.qa_findings.len() >= MAX_QA_FINDINGS {
        return Err(McpError::InvalidRequest(
            "creative QA finding capacity reached".into(),
        ));
    }
    project.qa_findings.push(finding);
    persist_project(cwd, config, project)
}

pub fn load_project(
    cwd: Option<&str>,
    config: &ServerConfig,
    project_id: &str,
) -> Result<CreativeProject, McpError> {
    validate_id(project_id, "project_id")?;
    let project: CreativeProject = read_json(cwd, config, &project_path(project_id))?;
    project.validate()?;
    Ok(project)
}

pub fn list_projects(cwd: Option<&str>, config: &ServerConfig) -> Result<Vec<String>, McpError> {
    Ok(read_project_index(cwd, config)?.project_ids)
}

fn persist_project(
    cwd: Option<&str>,
    config: &ServerConfig,
    mut project: CreativeProject,
) -> Result<CreativeProject, McpError> {
    project.updated_at_ms = now_ms();
    project.validate()?;
    write_json(
        cwd,
        config,
        &project_path(&project.project_id),
        &project,
        true,
    )?;
    Ok(project)
}
