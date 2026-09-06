use ai_tools::core::terminal_policy::validate_executable;

#[test]
fn generic_terminal_rejects_ssh_client_executables() {
    for binary in ["ssh", "scp", "sftp"] {
        let error = ai_tools::core::terminal_policy::validate_executable(binary, false)
            .expect_err("SSH client must use dedicated tool");
        assert!(error.to_string().contains("ssh_readonly_exec"));
    }
}

#[test]
fn privilege_escalation_commands_remain_forbidden() {
    for command in ["sudo", "su", "doas", "pkexec", "runas"] {
        assert!(validate_executable(command, true).is_err(), "{command}");
    }
}

#[test]
fn docker_requires_explicit_opt_in() {
    assert!(validate_executable("docker", false).is_err());
    assert!(validate_executable("docker", true).is_ok());
}

#[test]
fn executable_paths_remain_forbidden() {
    for path in [
        "/usr/bin/docker",
        "/bin/docker",
        "../docker",
        "./docker",
        "/usr/bin/sudo",
        "/bin/sh",
        "./script.sh",
        "nested/binary",
    ] {
        assert!(
            validate_executable(path, true).is_err(),
            "{path} must be forbidden"
        );
    }
}

#[test]
fn safe_executable_resolution_permits_developer_tools_and_blocks_brokers() {
    use ai_tools::application::execution::resolve_safe_executable;
    use ai_tools::core::config::ServerConfig;

    let config = ServerConfig::default();
    // Common developer runtimes from safe PATH
    for cmd in ["sh", "python3", "git"] {
        assert!(
            resolve_safe_executable(&config, cmd).is_ok(),
            "{cmd} should resolve from safe PATH"
        );
    }

    // Privilege brokers and SSH clients fail resolution even if present on host
    for forbidden in [
        "sudo", "su", "doas", "pkexec", "runas", "ssh", "scp", "sftp",
    ] {
        assert!(
            resolve_safe_executable(&config, forbidden).is_err(),
            "{forbidden} must fail safe resolution"
        );
    }
}
