            let mut parts = shell_words::split(command).unwrap_or_default();
            let binary = if !parts.is_empty() { parts[0].clone() } else { String::new() };
            
            if !binary.is_empty() {
                let binary_name = std::path::Path::new(&binary)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("");
                
                // 1. Explicitly reject forbidden commands
                let forbidden = ["sudo", "su", "doas", "pkexec", "runas"];
                if forbidden.contains(&binary_name) {
                    return Err(McpError::InvalidRequest(format!("execution of '{}' is forbidden", binary_name)));
                }
                
                // 2. Reject shell/interpreter bypasses
                let interpreters = ["sh", "bash", "zsh", "python", "python3", "node", "ruby", "perl", "cmd", "powershell", "pwsh"];
                if interpreters.contains(&binary_name) {
                    if parts.iter().any(|arg| arg == "-c" || arg == "-e" || arg == "--eval" || arg == "-Command" || arg == "/c") {
                        return Err(McpError::InvalidRequest(format!("shell/interpreter bypass is forbidden for '{}'", binary_name)));
                    }
                }
                
                // 3. Reject path traversal or absolute paths to prevent bypassing allowlist
                if binary.contains('/') || binary.contains('\\') || binary == ".." {
                    return Err(McpError::InvalidRequest("path traversal or absolute paths in executable name are forbidden".to_string()));
                }
                
                // 4. Server-controlled allowlist
                let allowed_commands = [
                    "npm", "npx", "node", "cargo", "rustc", "git", "ls", "cat", "grep", "rg", "pwd",
                    "echo", "head", "tail", "wc", "stat", "file", "tree", "diff", "find", "sed",
                    "python", "python3", "pip", "sh", "bash", "zsh", "env", "rustup", "make", "gcc", "g++", "cc", "c++",
                    "yarn", "pnpm", "nvm", "go", "jq", "curl", "wget", "tar", "gzip", "unzip", "cp", "mv", "rm", "mkdir", "rmdir", "touch"
                ];
                
                if !allowed_commands.contains(&binary_name) {
                    return Err(McpError::InvalidRequest(format!("command '{}' is not in the server-approved allowlist", binary_name)));
                }

                args.push("--allow-command".to_string());
                args.push(binary);
            }
