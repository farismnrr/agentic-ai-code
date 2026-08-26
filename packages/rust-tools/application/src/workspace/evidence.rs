use serde::Serialize;

const MAX_EVIDENCE_FILE_BYTES: usize = 512 * 1024;

#[derive(Debug, Serialize)]
pub(crate) struct ActivityEvidence {
    pub evidence: &'static str,
    pub complete: bool,
    pub preview: bool,
    pub change_type: &'static str,
    pub content_kind: &'static str,
    pub files: Vec<ActivityFileEvidence>,
}

#[derive(Debug, Serialize)]
pub(crate) struct ActivityFileEvidence {
    pub path: String,
    pub before: Option<String>,
    pub after: Option<String>,
}

pub(crate) fn activity_evidence(
    path: &str,
    before: Option<&[u8]>,
    after: &[u8],
) -> ActivityEvidence {
    let before_text = before.and_then(|bytes| String::from_utf8(bytes.to_vec()).ok());
    let after_text = String::from_utf8(after.to_vec()).ok();
    let changed = before.is_none_or(|bytes| bytes != after);
    let complete = !changed
        || (before.is_none_or(|bytes| bytes.len() <= MAX_EVIDENCE_FILE_BYTES)
            && after.len() <= MAX_EVIDENCE_FILE_BYTES
            && before_text.is_some()
            && after_text.is_some());
    ActivityEvidence {
        evidence: if !changed {
            "not_applicable"
        } else if complete {
            "exact"
        } else {
            "unavailable"
        },
        complete,
        preview: false,
        change_type: if before.is_none() { "create" } else { "modify" },
        content_kind: if complete { "text" } else { "binary" },
        files: vec![ActivityFileEvidence {
            path: path.to_owned(),
            before: complete.then_some(before_text).flatten(),
            after: complete.then_some(after_text).flatten(),
        }],
    }
}

impl ActivityEvidence {
    pub(crate) fn no_change() -> Self {
        Self {
            evidence: "not_applicable",
            complete: true,
            preview: false,
            change_type: "no_change",
            content_kind: "text",
            files: Vec::new(),
        }
    }
}
