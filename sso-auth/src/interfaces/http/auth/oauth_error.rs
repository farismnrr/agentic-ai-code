use axum::response::Response;

use super::super::{
    auth_response::clear_auth_flow_cookies, oauth, AuthHttpState,
};

pub(super) async fn oauth_error_redirect(
    state: &AuthHttpState,
    return_to: Option<&str>,
    error: &'static str,
) -> Option<Response> {
    let mut response =
        oauth::authorization_error_from_return_to(state, return_to?, error).await?;
    clear_auth_flow_cookies(&mut response, state);
    Some(response)
}
