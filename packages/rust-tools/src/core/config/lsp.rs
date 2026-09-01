use crate::core::error::RelayError;

pub(super) fn validate_entries(entries: &[String]) -> Result<(), RelayError> {
    if entries.len() > 16 {
        return Err(RelayError::InvalidConfig(
            "at most 16 lsp-server mappings may be configured".into(),
        ));
    }
    let mut languages = std::collections::HashSet::new();
    for entry in entries {
        let Some((language, executable)) = entry.split_once('=') else {
            return Err(RelayError::InvalidConfig(
                "lsp-server must use language=executable syntax".into(),
            ));
        };
        let valid_language = !language.is_empty()
            && language.len() <= 64
            && language.bytes().all(|byte| {
                byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'+')
            });
        let valid_executable = !executable.is_empty()
            && executable.len() <= 128
            && !executable.contains('/')
            && !executable.contains('\\')
            && executable.bytes().all(|byte| {
                byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'+')
            });
        if !valid_language || !valid_executable {
            return Err(RelayError::InvalidConfig(
                "lsp-server language/executable contains unsupported characters".into(),
            ));
        }
        if !languages.insert(language.to_ascii_lowercase()) {
            return Err(RelayError::InvalidConfig(
                "lsp-server language is configured more than once".into(),
            ));
        }
    }
    Ok(())
}
