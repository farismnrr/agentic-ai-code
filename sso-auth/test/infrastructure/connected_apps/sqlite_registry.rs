use super::*;

fn test_path() -> PathBuf {
    let name = format!(
        "sso-connected-apps-{}-{}.sqlite3",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    std::env::temp_dir().join(name)
}

#[test]
fn saves_reads_and_deletes_connected_apps() {
    let path = test_path();
    let repository = SqliteConnectedAppRepository::new(path.clone()).unwrap();
    let app = ConnectedApp {
        client_id: "relay-agent".to_string(),
        name: "Masih Awam Relay".to_string(),
        description: "Relay connection".to_string(),
        callback_url: "http://localhost:3100/connections/callback".to_string(),
        enabled: true,
        assertion_ttl_seconds: 90,
    };

    repository.save(app.clone()).unwrap();
    assert_eq!(repository.find("relay-agent").unwrap(), Some(app.clone()));
    assert_eq!(repository.list().unwrap(), vec![app]);
    assert!(repository.delete("relay-agent").unwrap());
    assert!(repository.find("relay-agent").unwrap().is_none());

    let _ = fs::remove_file(path);
}
