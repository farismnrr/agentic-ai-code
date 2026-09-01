//! The closed, explicitly reviewed `_vue:` command surface (Required fix
//! 2) — split out of `tsserver_bridge.rs` to stay under the maintainability
//! file-length budget.

use serde_json::Value;

/// Explicit allowlist of exact `_vue:` tsserver command names the
/// installed, reviewed `@vue/language-server@3.3.8` build actually sends
/// over the bridge. Verified two ways: every `sendTsServerRequest(...)`
/// call site in the installed `@vue/language-server/lib/server.js`, and
/// every `session.addProtocolHandler('_vue:...', ...)` registration in the
/// bundled `@vue/typescript-plugin@3.3.8`'s `index.js` (the two lists
/// match exactly). Any other command is never forwarded to the child
/// process; the bridge answers it with bounded `null` instead.
pub(super) const ALLOWED_COMMANDS: &[&str] = &[
    "_vue:projectInfo",
    "_vue:quickinfo",
    "_vue:documentHighlights-full",
    "_vue:encodedSemanticClassifications-full",
    "_vue:resolveModuleName",
    "_vue:collectExtractProps",
    "_vue:getComponentDirectives",
    "_vue:getComponentMeta",
    "_vue:getComponentNames",
    "_vue:getComponentProps",
    "_vue:getComponentSlots",
    "_vue:getElementAttrs",
    "_vue:getElementNames",
    "_vue:getImportPathForFile",
    "_vue:getAutoImportSuggestions",
    "_vue:resolveAutoImportCompletionEntry",
    "_vue:isRefAtPosition",
];

/// Commands whose bridged arguments are a positional array
/// `[fileName, ...]`, per `@vue/typescript-plugin@3.3.8`'s
/// `addProtocolHandler` bodies (each destructures `request.arguments` as
/// `const [fileName, ...] = request.arguments`). `resolveAutoImportCompletionEntry`
/// is deliberately excluded: its sole array element is an opaque
/// completion-entry data blob, not a file name.
const POSITIONAL_FILE_COMMANDS: &[&str] = &[
    "resolveModuleName",
    "collectExtractProps",
    "getComponentDirectives",
    "getComponentMeta",
    "getComponentNames",
    "getComponentProps",
    "getComponentSlots",
    "getElementAttrs",
    "getElementNames",
    "getImportPathForFile",
    "getAutoImportSuggestions",
    "isRefAtPosition",
];

/// Extracts the file-path argument (if any) that must be resolved through
/// the contained-path + protected-path policy before this command is
/// forwarded. `None` means the command carries no file argument to
/// validate (currently only `resolveAutoImportCompletionEntry`).
pub(super) fn extract_file_argument(command: &str, arguments: &Value) -> Option<String> {
    let command = command.strip_prefix("_vue:").unwrap_or(command);
    if POSITIONAL_FILE_COMMANDS.contains(&command) {
        arguments.get(0).and_then(Value::as_str).map(str::to_owned)
    } else {
        arguments
            .get("file")
            .and_then(Value::as_str)
            .map(str::to_owned)
    }
}
