use crate::core::error::McpError;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process;

pub struct Pidfile {
    path: PathBuf,
}

impl Pidfile {
    pub fn new(port: u16) -> Result<Self, McpError> {
        let path = std::env::temp_dir().join(format!("relay-agent-{port}.pid"));

        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(pid) = content.trim().parse::<i32>() {
                    // Check if process is alive (Unix only)
                    #[cfg(unix)]
                    {
                        if unsafe { libc::kill(pid, 0) } == 0 {
                            return Err(McpError::Internal(format!(
                                "relay-agent is already running on port {} with PID {}",
                                port, pid
                            )));
                        }
                    }
                }
            }
            let _ = fs::remove_file(&path);
        }

        let pid = process::id();
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|_| McpError::Internal("Failed to create pidfile".to_string()))?;

        writeln!(file, "{}", pid)
            .map_err(|_| McpError::Internal("Failed to write pidfile".to_string()))?;

        Ok(Self { path })
    }

    pub fn stop(port: u16) -> Result<(), String> {
        let path = std::env::temp_dir().join(format!("relay-agent-{port}.pid"));
        if !path.exists() {
            return Err(format!("pidfile not found: {}", path.display()));
        }

        let content =
            fs::read_to_string(&path).map_err(|_| "Failed to read pidfile".to_string())?;

        let pid = content
            .trim()
            .parse::<i32>()
            .map_err(|_| "Invalid PID in pidfile".to_string())?;

        #[cfg(unix)]
        {
            if unsafe { libc::kill(pid, libc::SIGTERM) } == 0 {
                println!("Sent SIGTERM to process {}", pid);
            } else {
                return Err("Process not found or cannot be killed".to_string());
            }
        }
        #[cfg(not(unix))]
        {
            return Err("stop is only supported on Unix".to_string());
        }

        let _ = fs::remove_file(&path);
        Ok(())
    }
}

impl Drop for Pidfile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}
