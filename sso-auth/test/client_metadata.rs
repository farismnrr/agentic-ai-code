use url::Url;

use crate::infrastructure::mcp_oauth::trusted_chatgpt_client_id;

#[test]
fn trusts_stable_and_callback_specific_chatgpt_cimd_urls() {
    for value in [
        "https://chatgpt.com/oauth/client.json",
        "https://chatgpt.com/oauth/callback-123/client.json",
    ] {
        assert!(trusted_chatgpt_client_id(&Url::parse(value).unwrap()));
    }
}

#[test]
fn rejects_non_chatgpt_or_redirectable_client_ids() {
    for value in [
        "https://example.com/oauth/client.json",
        "https://chatgpt.com/",
        "https://chatgpt.com/oauth/client.json?next=x",
        "https://chatgpt.com/oauth/a/b/client.json",
        "http://chatgpt.com/oauth/client.json",
    ] {
        assert!(!trusted_chatgpt_client_id(&Url::parse(value).unwrap()));
    }
}
