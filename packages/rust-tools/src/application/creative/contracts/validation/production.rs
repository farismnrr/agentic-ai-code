use super::*;

pub(super) fn validate_scene_board(
    board: &SceneBoard,
    project: &CreativeProject,
) -> Result<(), McpError> {
    validate_id(&board.board_id, "scene board id")?;
    validate_id(&board.scene_id, "scene board scene id")?;
    if let Some(revision_id) = board.revision_id.as_deref() {
        validate_id(revision_id, "scene board revision id")?;
    }
    if board.frames.len() > MAX_SCENE_BOARD_FRAMES {
        return Err(McpError::InvalidRequest(
            "scene board frame count exceeds maximum".into(),
        ));
    }
    let mut ids = HashSet::new();
    let mut orders = HashSet::new();
    for frame in &board.frames {
        validate_id(&frame.frame_id, "scene board frame id")?;
        if !ids.insert(frame.frame_id.as_str()) || !orders.insert(frame.order) {
            return Err(McpError::InvalidRequest(
                "scene board frame identity/order is duplicated".into(),
            ));
        }
        if let Some(revision_id) = frame.revision_id.as_deref() {
            validate_id(revision_id, "scene board frame revision id")?;
        }
        if let Some(shot_id) = frame.shot_id.as_deref() {
            validate_id(shot_id, "scene board shot id")?;
        }
        validate_spec(&frame.direction)?;
        if frame.element_ids.len() > MAX_REFERENCES_PER_REVISION
            || frame.reference_asset_ids.len() > MAX_REFERENCES_PER_REVISION
        {
            return Err(McpError::InvalidRequest(
                "scene board frame references exceed maximum".into(),
            ));
        }
        for element_id in &frame.element_ids {
            if project.element(element_id).is_none() {
                return Err(McpError::InvalidRequest(
                    "scene board references an unknown element".into(),
                ));
            }
        }
        for asset_id in &frame.reference_asset_ids {
            if project.asset(asset_id).is_none() {
                return Err(McpError::InvalidRequest(
                    "scene board references an unknown asset".into(),
                ));
            }
        }
    }
    Ok(())
}

pub(super) fn validate_scene(
    scene: &SceneManifest,
    project: &CreativeProject,
) -> Result<(), McpError> {
    validate_id(&scene.scene_id, "scene_id")?;
    validate_text(&scene.title, 1, 200, "scene title")?;
    if let Some(revision_id) = scene.revision_id.as_deref() {
        validate_id(revision_id, "scene revision id")?;
    }
    if scene
        .target_duration_ms
        .is_some_and(|duration| duration == 0 || duration > 86_400_000)
    {
        return Err(McpError::InvalidRequest(
            "scene target duration is outside allowed bounds".into(),
        ));
    }
    validate_scene_direction(&scene.global_direction)?;
    if scene.shots.len() > MAX_SCENE_SHOTS {
        return Err(McpError::InvalidRequest(
            "scene shot count exceeds maximum".into(),
        ));
    }
    let mut shot_ids = HashSet::new();
    let mut shot_orders = HashSet::new();
    for element_id in scene
        .cast_element_ids
        .iter()
        .chain(scene.style_element_id.iter())
    {
        if project.element(element_id).is_none() {
            return Err(McpError::InvalidRequest(
                "scene references an unknown element".into(),
            ));
        }
    }
    if let Some(hero_asset_id) = scene.selected_hero_frame_asset_id.as_deref() {
        validate_id(hero_asset_id, "scene hero frame asset id")?;
        if project.asset(hero_asset_id).is_none() {
            return Err(McpError::InvalidRequest(
                "scene hero frame references an unknown asset".into(),
            ));
        }
    }
    let location_pack = scene
        .location_element_id
        .as_deref()
        .map(|element_id| location::selected_location_pack(project, element_id))
        .transpose()?;
    let mut total_duration_ms = 0u64;
    for shot in &scene.shots {
        validate_id(&shot.shot_id, "shot_id")?;
        if let Some(revision_id) = shot.revision_id.as_deref() {
            validate_id(revision_id, "shot revision id")?;
        }
        if !shot_ids.insert(shot.shot_id.as_str())
            || !shot_orders.insert(shot.order)
            || shot.duration_ms == 0
            || shot.duration_ms > 3_600_000
        {
            return Err(McpError::InvalidRequest(
                "scene shot identity or duration is invalid".into(),
            ));
        }
        total_duration_ms = total_duration_ms.saturating_add(shot.duration_ms);
        validate_freeform_text(&shot.action, 0, 4_096, "shot action")?;
        validate_spec(&shot.state_in)?;
        validate_spec(&shot.state_out)?;
        validate_spec(&shot.continuity)?;
        validate_camera_spec(&shot.camera)?;
        validate_director_shot_spec(&shot.director)?;
        if let Some(engine) = shot.engine.as_deref() {
            if !matches!(engine, "generated_video" | "blender" | "mixed") {
                return Err(McpError::InvalidRequest(
                    "shot engine is unsupported".into(),
                ));
            }
        }
        if let Some(binding_id) = shot.execution_binding_id.as_deref() {
            validate_id(binding_id, "shot execution binding id")?;
        }
        if shot.reference_asset_ids.len() > 32 {
            return Err(McpError::InvalidRequest(
                "shot reference asset count exceeds maximum".into(),
            ));
        }
        for asset_id in shot
            .reference_asset_ids
            .iter()
            .chain(shot.hero_frame_asset_id.iter())
            .chain(shot.carry_forward_anchor_asset_id.iter())
        {
            validate_id(asset_id, "shot asset id")?;
            if project.asset(asset_id).is_none() {
                return Err(McpError::InvalidRequest(
                    "shot references an unknown asset".into(),
                ));
            }
        }
        if let Some(variant_id) = shot.location_variant_id.as_deref() {
            validate_id(variant_id, "shot location variant id")?;
            let pack = location_pack.as_ref().ok_or_else(|| {
                McpError::InvalidRequest(
                    "shot location variant requires a scene Location Element".into(),
                )
            })?;
            if !pack
                .variants
                .iter()
                .any(|variant| variant.variant_id == variant_id)
            {
                return Err(McpError::InvalidRequest(
                    "shot location variant is not declared by the selected Location Pack".into(),
                ));
            }
        }
        for element_id in &shot.element_ids {
            if project.element(element_id).is_none() {
                return Err(McpError::InvalidRequest(
                    "shot references an unknown element".into(),
                ));
            }
        }
    }
    if scene
        .target_duration_ms
        .is_some_and(|target| total_duration_ms > target.saturating_mul(2))
    {
        return Err(McpError::InvalidRequest(
            "scene shot durations materially exceed the declared target".into(),
        ));
    }
    Ok(())
}

fn validate_scene_direction(direction: &SceneGlobalDirection) -> Result<(), McpError> {
    for (value, label) in [
        (direction.genre_look.as_deref(), "scene genre/look"),
        (direction.lighting.as_deref(), "scene lighting"),
        (direction.atmosphere.as_deref(), "scene atmosphere"),
        (direction.era_time.as_deref(), "scene era/time"),
    ] {
        if let Some(value) = value {
            validate_freeform_text(value, 1, 2_048, label)?;
        }
    }
    if direction.color_palette.len() > 32 {
        return Err(McpError::InvalidRequest(
            "scene color palette exceeds allowed bounds".into(),
        ));
    }
    for color in &direction.color_palette {
        validate_text(color, 1, 128, "scene palette entry")?;
    }
    validate_spec(&direction.spatial_constraints)
}

fn validate_camera_spec(camera: &CameraSpec) -> Result<(), McpError> {
    for (value, label) in [
        (camera.shot_size.as_deref(), "shot size"),
        (camera.movement.as_deref(), "camera movement"),
        (camera.framing.as_deref(), "camera framing"),
    ] {
        if let Some(value) = value {
            validate_freeform_text(value, 1, 512, label)?;
        }
    }
    if camera
        .focal_length_mm
        .is_some_and(|value| !value.is_finite() || !(1.0..=2000.0).contains(&value))
        || camera
            .aperture_f
            .is_some_and(|value| !value.is_finite() || !(0.1..=64.0).contains(&value))
    {
        return Err(McpError::InvalidRequest(
            "shot camera optics are outside allowed bounds".into(),
        ));
    }
    Ok(())
}

fn validate_director_shot_spec(director: &DirectorShotSpec) -> Result<(), McpError> {
    for (value, label) in [
        (
            director.camera_profile.as_deref(),
            "director camera profile",
        ),
        (
            director.depth_of_field_intent.as_deref(),
            "director depth-of-field intent",
        ),
        (
            director.movement_speed.as_deref(),
            "director movement speed",
        ),
        (director.stabilization.as_deref(), "director stabilization"),
        (
            director.tempo_edit_intent.as_deref(),
            "director tempo/edit intent",
        ),
    ] {
        if let Some(value) = value {
            validate_freeform_text(value, 1, 512, label)?;
        }
    }
    Ok(())
}

pub(super) fn validate_game(
    game: &GameManifest,
    project: &CreativeProject,
) -> Result<(), McpError> {
    validate_id(&game.game_id, "game_id")?;
    if let Some(revision_id) = game.revision_id.as_deref() {
        validate_id(revision_id, "game revision id")?;
    }
    for (value, label) in [
        (&game.title, "game title"),
        (&game.genre, "game genre"),
        (&game.perspective, "game perspective"),
    ] {
        validate_text(value, 1, 4_096, label)?;
    }
    for (value, label) in [
        (&game.core_loop, "game core loop"),
        (&game.win_condition, "game win condition"),
        (&game.lose_condition, "game lose condition"),
        (&game.restart_behavior, "game restart behavior"),
    ] {
        validate_freeform_text(value, 1, 4_096, label)?;
    }
    for (value, label) in [
        (game.progression.as_deref(), "game progression"),
        (game.camera.as_deref(), "game camera"),
        (game.language.as_deref(), "game language"),
        (
            game.placeholder_policy.as_deref(),
            "game placeholder policy",
        ),
    ] {
        if let Some(value) = value {
            validate_freeform_text(value, 1, 2_048, label)?;
        }
    }
    validate_spec(&game.physics_timing)?;
    validate_spec(&game.style_formula)?;
    if let Some(style_id) = game.style_element_id.as_deref() {
        if project.element(style_id).is_none() {
            return Err(McpError::InvalidRequest(
                "game style references an unknown element".into(),
            ));
        }
    }
    if game.target_devices.len() > MAX_GAME_LIST_ITEMS
        || game.verbs.len() > MAX_GAME_LIST_ITEMS
        || game.inputs.len() > MAX_GAME_LIST_ITEMS
        || game.asset_roles.len() > MAX_GAME_ASSET_ROLES
    {
        return Err(McpError::InvalidRequest(
            "game manifest collection exceeds allowed bounds".into(),
        ));
    }
    if game
        .runtime_budget
        .target_fps
        .is_some_and(|fps| fps == 0 || fps > 240)
        || game
            .runtime_budget
            .max_asset_bytes
            .is_some_and(|bytes| bytes == 0 || bytes > 4 * 1024 * 1024 * 1024)
        || game
            .runtime_budget
            .max_initial_load_ms
            .is_some_and(|ms| ms == 0 || ms > 300_000)
    {
        return Err(McpError::InvalidRequest(
            "game runtime budget is outside allowed bounds".into(),
        ));
    }
    for value in game
        .target_devices
        .iter()
        .chain(game.verbs.iter())
        .chain(game.inputs.iter())
    {
        validate_text(value, 1, 128, "game manifest list item")?;
    }
    let mut roles = HashSet::new();
    let mut runtime_paths = HashSet::new();
    for role in &game.asset_roles {
        validate_text(&role.role, 1, 128, "game asset role")?;
        validate_relative_path(&role.runtime_path, "game runtime asset path")?;
        if !roles.insert(role.role.as_str()) || !runtime_paths.insert(role.runtime_path.as_str()) {
            return Err(McpError::InvalidRequest(
                "game asset roles and runtime paths must be unique".into(),
            ));
        }
        if let Some(asset_id) = role.asset_id.as_deref() {
            validate_id(asset_id, "game asset id")?;
            if project.asset(asset_id).is_none() {
                return Err(McpError::InvalidRequest(
                    "game asset role references an unknown asset".into(),
                ));
            }
        }
    }
    for (value, label) in [
        (
            game.build_state.source_revision.as_deref(),
            "game source revision",
        ),
        (
            game.build_state.build_revision.as_deref(),
            "game build revision",
        ),
        (
            game.build_state.accepted_build_revision.as_deref(),
            "game accepted build revision",
        ),
        (
            game.build_state.deployment_id.as_deref(),
            "game deployment id",
        ),
    ] {
        if let Some(value) = value {
            validate_id(value, label)?;
        }
    }
    if let Some(url) = game.build_state.deployment_url.as_deref() {
        validate_text(url, 1, 2_048, "game deployment URL")?;
    }
    if game.build_state.published {
        return Err(McpError::InvalidRequest(
            "public game publication is not part of the Creative Project build/deploy state".into(),
        ));
    }
    Ok(())
}

pub(super) fn validate_audio(audio: &AudioPlan, project: &CreativeProject) -> Result<(), McpError> {
    validate_id(&audio.audio_plan_id, "audio_plan_id")?;
    if let Some(voice_id) = audio.voice_element_id.as_deref() {
        if project.element(voice_id).is_none() {
            return Err(McpError::InvalidRequest(
                "audio plan voice references an unknown element".into(),
            ));
        }
    }
    if audio.cues.len() > MAX_AUDIO_CUES {
        return Err(McpError::InvalidRequest(
            "audio cue count exceeds maximum".into(),
        ));
    }
    let mut cue_ids = HashSet::new();
    for cue in &audio.cues {
        validate_id(&cue.cue_id, "audio cue id")?;
        if !cue_ids.insert(cue.cue_id.as_str()) {
            return Err(McpError::InvalidRequest(
                "duplicate audio cue identity".into(),
            ));
        }
        if let Some(asset_id) = cue.asset_id.as_deref() {
            if project.asset(asset_id).is_none() {
                return Err(McpError::InvalidRequest(
                    "audio cue references an unknown asset".into(),
                ));
            }
        }
        if let Some(text) = cue.text.as_deref() {
            validate_freeform_text(text, 1, 4_096, "audio cue text")?;
        }
    }
    Ok(())
}
