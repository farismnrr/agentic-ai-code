use super::super::contracts::AssetSource;
use crate::core::error::McpError;
use ring::digest::{digest, SHA256};

const MAX_FILENAME_BYTES: usize = 180;

pub(super) fn validate_upload_source(source: &AssetSource) -> Result<(), McpError> {
    if matches!(
        source,
        AssetSource::McpUpload | AssetSource::ConversationUpload
    ) {
        Ok(())
    } else {
        Err(McpError::InvalidRequest(
            "creative upload source must be mcp_upload or conversation_upload".into(),
        ))
    }
}

pub(super) fn validate_owner(owner: &str) -> Result<(), McpError> {
    if owner.is_empty() || owner.len() > 512 || owner.chars().any(char::is_control) {
        return Err(McpError::InvalidRequest(
            "creative owner identity exceeds allowed bounds".into(),
        ));
    }
    Ok(())
}

pub(super) fn validate_media_type(value: &str) -> Result<(), McpError> {
    let value = normalize_content_type(value);
    if value.is_empty()
        || value.len() > 128
        || value.chars().any(char::is_control)
        || !(value.starts_with("image/")
            || value.starts_with("video/")
            || value.starts_with("model/")
            || matches!(
                value.as_str(),
                "application/octet-stream" | "application/x-blender"
            ))
    {
        return Err(McpError::InvalidRequest(
            "creative media type is unsupported".into(),
        ));
    }
    Ok(())
}

pub(super) fn validate_role(value: &str) -> Result<(), McpError> {
    if value.is_empty() || value.len() > 128 || value.chars().any(char::is_control) {
        return Err(McpError::InvalidRequest(
            "creative media role exceeds allowed bounds".into(),
        ));
    }
    Ok(())
}

pub(super) fn validate_untrusted_media_bytes(
    media_type: &str,
    bytes: &[u8],
) -> Result<(), McpError> {
    let media_type = normalize_content_type(media_type);
    let matches = match media_type.as_str() {
        "image/png" => bytes.starts_with(b"\x89PNG\r\n\x1a\n"),
        "image/jpeg" => bytes.starts_with(&[0xff, 0xd8, 0xff]),
        "image/webp" => bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP",
        "image/gif" => bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a"),
        "image/svg+xml" => {
            return Err(McpError::InvalidRequest(
                "untrusted SVG media is not accepted without a reviewed sanitizer".into(),
            ))
        }
        "video/mp4" => bytes.len() >= 12 && &bytes[4..8] == b"ftyp",
        "model/gltf-binary" => bytes.starts_with(b"glTF"),
        "application/x-blender" => bytes.starts_with(b"BLENDER"),
        "application/octet-stream" => true,
        _ => {
            return Err(McpError::InvalidRequest(
                "untrusted creative media type has no reviewed byte validator".into(),
            ))
        }
    };
    if !matches {
        return Err(McpError::InvalidRequest(
            "creative media bytes do not match the declared content type".into(),
        ));
    }
    Ok(())
}

pub(super) fn sanitize_filename(value: &str) -> Result<String, McpError> {
    if value.is_empty() || value.len() > MAX_FILENAME_BYTES || value.chars().any(char::is_control) {
        return Err(McpError::InvalidRequest(
            "creative media filename exceeds allowed bounds".into(),
        ));
    }
    if value.contains('/') || value.contains('\\') || value == "." || value == ".." {
        return Err(McpError::InvalidRequest(
            "creative media filename must be a plain filename".into(),
        ));
    }
    let sanitized = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_') {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>();
    if sanitized.starts_with('.') || sanitized.is_empty() {
        return Err(McpError::InvalidRequest(
            "creative media filename is not allowed".into(),
        ));
    }
    Ok(sanitized)
}

pub(super) fn normalize_content_type(value: &str) -> String {
    value
        .split(';')
        .next()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase()
}

pub(super) fn filename_from_url(url: &reqwest::Url, media_type: &str) -> String {
    if let Some(segment) = url
        .path_segments()
        .and_then(|mut segments| segments.next_back())
    {
        if let Ok(filename) = sanitize_filename(segment) {
            return filename;
        }
    }
    let extension = match media_type {
        "image/png" => "png",
        "image/jpeg" => "jpg",
        "image/webp" => "webp",
        "video/mp4" => "mp4",
        "model/gltf+json" => "gltf",
        "model/gltf-binary" => "glb",
        _ => "bin",
    };
    format!("import.{extension}")
}

pub(super) fn sha256_text(value: &str) -> String {
    let bytes = digest(&SHA256, value.as_bytes());
    let mut output = String::with_capacity(64);
    for byte in bytes.as_ref() {
        output.push_str(&format!("{byte:02x}"));
    }
    output
}
