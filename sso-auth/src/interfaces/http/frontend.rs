use axum::{
    body::Body,
    http::{header::CONTENT_TYPE, StatusCode, Uri},
    response::{IntoResponse, Response},
};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "dist/"]
struct FrontendAssets;

pub async fn serve(uri: Uri) -> Response {
    let requested = normalized_path(&uri);

    if let Some(response) = embedded_response(requested) {
        return response;
    }

    if should_use_spa_fallback(requested) {
        if let Some(response) = embedded_response("index.html") {
            return response;
        }
    }

    StatusCode::NOT_FOUND.into_response()
}

fn normalized_path(uri: &Uri) -> &str {
    let path = uri.path().trim_start_matches('/');
    if path.is_empty() {
        "index.html"
    } else {
        path
    }
}

fn should_use_spa_fallback(path: &str) -> bool {
    !path.starts_with(".well-known/") && !path.rsplit('/').next().unwrap_or(path).contains('.')
}

fn embedded_response(path: &str) -> Option<Response> {
    let asset = FrontendAssets::get(path)?;
    let content_type = mime_guess::from_path(path).first_or_octet_stream();

    Some(
        Response::builder()
            .status(StatusCode::OK)
            .header(CONTENT_TYPE, content_type.as_ref())
            .body(Body::from(asset.data.into_owned()))
            .expect("embedded frontend response must be valid"),
    )
}

#[cfg(test)]
mod tests {
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
}
