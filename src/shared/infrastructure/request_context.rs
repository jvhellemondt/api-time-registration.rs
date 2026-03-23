use axum::extract::FromRequestParts;
use axum::http::StatusCode;
use axum::http::request::Parts;

pub struct RequestContext {
    pub user_id: String,
    pub tenant_id: String,
}

impl<S: Send + Sync> FromRequestParts<S> for RequestContext {
    type Rejection = StatusCode;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, StatusCode> {
        let user_id = parts
            .headers
            .get("x-user-id")
            .and_then(|v| v.to_str().ok())
            .map(str::to_string)
            .ok_or(StatusCode::UNAUTHORIZED)?;
        let tenant_id = parts
            .headers
            .get("x-tenant-id")
            .and_then(|v| v.to_str().ok())
            .map(str::to_string)
            .ok_or(StatusCode::UNAUTHORIZED)?;
        Ok(RequestContext { user_id, tenant_id })
    }
}

#[cfg(test)]
mod request_context_tests {
    use axum::{
        Router,
        body::Body,
        http::{Request, StatusCode},
        routing::get,
    };
    use tower::ServiceExt;

    use super::RequestContext;

    async fn handler(ctx: RequestContext) -> String {
        format!("{}:{}", ctx.user_id, ctx.tenant_id)
    }

    fn app() -> Router {
        Router::new().route("/", get(handler))
    }

    #[tokio::test]
    async fn extracts_both_headers_successfully() {
        let response = app()
            .oneshot(
                Request::get("/")
                    .header("x-user-id", "u-1")
                    .header("x-tenant-id", "t-1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn returns_401_when_user_id_missing() {
        let response = app()
            .oneshot(
                Request::get("/")
                    .header("x-tenant-id", "t-1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn returns_401_when_tenant_id_missing() {
        let response = app()
            .oneshot(
                Request::get("/")
                    .header("x-user-id", "u-1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
}
