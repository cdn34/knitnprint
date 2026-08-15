use axum::{
    Json,
    extract::{Request, State},
    http::{HeaderMap, HeaderValue, Method, header},
    middleware::Next,
    response::{IntoResponse, Response},
};

use crate::error::ErrorBody;

#[derive(Clone, Debug)]
pub struct SecurityPolicy {
    pub allowed_origins: Vec<String>,
    pub production: bool,
}

impl Default for SecurityPolicy {
    fn default() -> Self {
        Self {
            allowed_origins: vec![
                "http://127.0.0.1:3000".into(),
                "http://localhost:3000".into(),
                "http://127.0.0.1:3001".into(),
                "http://localhost:3001".into(),
            ],
            production: false,
        }
    }
}

pub async fn enforce(
    State(policy): State<SecurityPolicy>,
    request: Request,
    next: Next,
) -> Response {
    if is_unsafe(request.method()) && !same_origin_request(request.headers(), &policy) {
        return (
            axum::http::StatusCode::FORBIDDEN,
            Json(ErrorBody::new(
                "cross_origin_request_rejected",
                "This state-changing request is not allowed from that origin.",
            )),
        )
            .into_response();
    }

    let mut response = next.run(request).await;
    let headers = response.headers_mut();
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(header::X_FRAME_OPTIONS, HeaderValue::from_static("DENY"));
    headers.insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("no-referrer"),
    );
    headers.insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static("default-src 'none'; frame-ancestors 'none'; sandbox"),
    );
    headers.insert(
        header::HeaderName::from_static("permissions-policy"),
        HeaderValue::from_static("camera=(), microphone=(), geolocation=(), payment=()"),
    );
    headers.insert(
        header::HeaderName::from_static("cross-origin-resource-policy"),
        HeaderValue::from_static("same-origin"),
    );
    if policy.production {
        headers.insert(
            header::STRICT_TRANSPORT_SECURITY,
            HeaderValue::from_static("max-age=31536000; includeSubDomains"),
        );
    }
    response
}

fn is_unsafe(method: &Method) -> bool {
    !matches!(method, &Method::GET | &Method::HEAD | &Method::OPTIONS)
}

fn same_origin_request(headers: &HeaderMap, policy: &SecurityPolicy) -> bool {
    if headers
        .get("sec-fetch-site")
        .is_some_and(|value| value == "cross-site")
    {
        return false;
    }
    let Some(origin) = headers.get(header::ORIGIN) else {
        // Non-browser clients and signed provider webhooks generally omit Origin.
        return true;
    };
    let Ok(origin) = origin.to_str() else {
        return false;
    };
    policy
        .allowed_origins
        .iter()
        .any(|allowed| allowed == origin)
}

#[cfg(test)]
mod tests {
    use axum::http::{HeaderMap, HeaderValue, Method};

    use super::{SecurityPolicy, is_unsafe, same_origin_request};

    #[test]
    fn unsafe_methods_are_origin_checked() {
        assert!(is_unsafe(&Method::POST));
        assert!(is_unsafe(&Method::PATCH));
        assert!(!is_unsafe(&Method::GET));
        assert!(!is_unsafe(&Method::OPTIONS));
    }

    #[test]
    fn configured_origins_pass_and_cross_site_requests_fail() {
        let policy = SecurityPolicy::default();
        let mut headers = HeaderMap::new();
        headers.insert("origin", HeaderValue::from_static("http://localhost:3001"));
        assert!(same_origin_request(&headers, &policy));

        headers.insert(
            "origin",
            HeaderValue::from_static("https://attacker.example"),
        );
        assert!(!same_origin_request(&headers, &policy));

        headers.remove("origin");
        headers.insert("sec-fetch-site", HeaderValue::from_static("cross-site"));
        assert!(!same_origin_request(&headers, &policy));
    }
}
