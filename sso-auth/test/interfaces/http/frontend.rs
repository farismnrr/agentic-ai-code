use super::should_use_spa_fallback;

#[test]
fn well_known_paths_never_fall_back_to_spa() {
    assert!(!should_use_spa_fallback(".well-known/openid-configuration"));
    assert!(!should_use_spa_fallback(
        ".well-known/oauth-authorization-server/tenant"
    ));
}

#[test]
fn ordinary_frontend_routes_still_use_spa_fallback() {
    assert!(should_use_spa_fallback("dashboard"));
    assert!(should_use_spa_fallback("settings/apps"));
}
