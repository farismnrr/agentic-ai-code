use super::*;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct UnvalidatedActivityEvent {
    contract_version: String,
    activity_id: String,
    source_id: String,
    source_sequence: u64,
    status: Status,
    tool_id: String,
    category: Category,
    effects: Vec<Effect>,
    workspace_root_fingerprint: Option<String>,
    actor: Actor,
    client_info: Option<ClientInfo>,
    occurred_at_ms: i64,
    ingested_at_ms: Option<i64>,
    duration_ms: Option<u64>,
    presentation: Presentation,
}

impl<'de> Deserialize<'de> for ActivityEvent {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = UnvalidatedActivityEvent::deserialize(deserializer)?;
        let event = Self {
            contract_version: raw.contract_version,
            activity_id: raw.activity_id,
            source_id: raw.source_id,
            source_sequence: raw.source_sequence,
            status: raw.status,
            tool_id: raw.tool_id,
            category: raw.category,
            effects: raw.effects,
            workspace_root_fingerprint: raw.workspace_root_fingerprint,
            actor: raw.actor,
            client_info: raw.client_info,
            occurred_at_ms: raw.occurred_at_ms,
            ingested_at_ms: raw.ingested_at_ms,
            duration_ms: raw.duration_ms,
            presentation: raw.presentation,
        };
        event.validate().map_err(serde::de::Error::custom)?;
        Ok(event)
    }
}

impl ActivityEvent {
    pub fn with_status(&self, status: Status, duration_ms: Option<u64>) -> Self {
        let mut next = self.clone();
        next.status = status;
        next.duration_ms = duration_ms;
        next
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.contract_version != CONTRACT_VERSION {
            return Err("unknown contract version".into());
        }
        bounded(&self.activity_id, MAX_TEXT, "activity_id")?;
        bounded(&self.source_id, MAX_TEXT, "source_id")?;
        bounded(&self.tool_id, MAX_TEXT, "tool_id")?;
        if self.source_sequence == 0 {
            return Err("source_sequence must be positive".into());
        }
        if self.effects.len() > MAX_LIST {
            return Err("too many effects".into());
        }
        if let Some(root) = &self.workspace_root_fingerprint {
            bounded(root, 128, "workspace_root_fingerprint")?;
        }
        bounded(&self.actor.label, MAX_TEXT, "actor.label")?;
        for (name, value) in [
            ("actor.source", &self.actor.source),
            ("actor.channel", &self.actor.channel),
        ] {
            if let Some(value) = value {
                bounded(value, MAX_TEXT, name)?;
            }
        }
        if let Some(client_info) = &self.client_info {
            bounded(&client_info.name, MAX_TEXT, "client_info.name")?;
            bounded(&client_info.version, 64, "client_info.version")?;
        }
        for (name, value) in [
            ("target", &self.presentation.target),
            ("summary", &self.presentation.summary),
            ("result_class", &self.presentation.result_class),
            ("payload_reference", &self.presentation.payload_reference),
        ] {
            if let Some(value) = value {
                bounded(
                    value,
                    if name == "target" { MAX_PATH } else { MAX_TEXT },
                    name,
                )?;
            }
        }
        if !legal_status(self.status) {
            return Err("illegal lifecycle status".into());
        }
        Ok(())
    }
}

fn bounded(value: &str, max: usize, name: &str) -> Result<(), String> {
    if value.is_empty() || value.len() > max || value.chars().any(char::is_control) {
        return Err(format!("invalid {name}"));
    }
    Ok(())
}

fn legal_status(status: Status) -> bool {
    matches!(
        status,
        Status::Started
            | Status::Running
            | Status::Ok
            | Status::Error
            | Status::Denied
            | Status::Cancelled
            | Status::Interrupted
    )
}

pub fn transition_allowed(from: Status, to: Status) -> bool {
    match from {
        Status::Started => matches!(
            to,
            Status::Started
                | Status::Running
                | Status::Ok
                | Status::Error
                | Status::Denied
                | Status::Cancelled
                | Status::Interrupted
        ),
        Status::Running => matches!(
            to,
            Status::Running
                | Status::Ok
                | Status::Error
                | Status::Denied
                | Status::Cancelled
                | Status::Interrupted
        ),
        _ => to == from,
    }
}
