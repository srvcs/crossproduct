use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use utoipa::{OpenApi, ToSchema};

use crate::client::{self, DepError};

pub const SERVICE: &str = "srvcs-crossproduct";
pub const CONCERN: &str = "vectors: 3D cross product";
pub const DEPENDS_ON: &[&str] = &["srvcs-floatmultiply", "srvcs-floatsubtract"];

/// Dependency endpoints, injected as router state so tests can point them at
/// mock services.
#[derive(Clone)]
pub struct Deps {
    pub floatmultiply_url: String,
    pub floatsubtract_url: String,
}

#[derive(Serialize, ToSchema)]
pub struct Info {
    pub service: &'static str,
    pub concern: &'static str,
    pub depends_on: Vec<&'static str>,
}

/// `GET /` — service identity (srvcs service standard).
#[utoipa::path(get, path = "/", responses((status = 200, body = Info)))]
pub async fn index() -> Json<Info> {
    Json(Info {
        service: SERVICE,
        concern: CONCERN,
        depends_on: DEPENDS_ON.to_vec(),
    })
}

#[derive(Deserialize, ToSchema)]
pub struct EvalRequest {
    /// The first 3D vector `[x, y, z]`. Each element is forwarded verbatim to
    /// the float primitives.
    #[schema(value_type = Object)]
    pub a: Vec<Value>,
    /// The second 3D vector `[x, y, z]`.
    #[schema(value_type = Object)]
    pub b: Vec<Value>,
}

#[derive(Serialize, ToSchema)]
pub struct CrossProductResponse {
    #[schema(value_type = Object)]
    pub a: Vec<Value>,
    #[schema(value_type = Object)]
    pub b: Vec<Value>,
    /// The cross product `a × b` as `[cx, cy, cz]`.
    #[schema(value_type = Object)]
    pub result: Vec<f64>,
}

fn ok(a: Vec<Value>, b: Vec<Value>, result: Vec<f64>) -> Response {
    (
        StatusCode::OK,
        Json(json!({ "a": a, "b": b, "result": result })),
    )
        .into_response()
}

fn degraded(dependency: &str) -> Response {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(json!({ "error": "dependency unavailable", "dependency": dependency })),
    )
        .into_response()
}

fn forward(status: u16, body: Value) -> Response {
    let code = StatusCode::from_u16(status).unwrap_or(StatusCode::BAD_GATEWAY);
    (code, Json(body)).into_response()
}

/// A reachable dependency answered `200` but its body lacked a numeric
/// `result`. That is a contract violation we cannot recover from, so surface a
/// `500` rather than guessing.
fn malformed(dependency: &str) -> Response {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(
            json!({ "error": "dependency returned a malformed result", "dependency": dependency }),
        ),
    )
        .into_response()
}

/// Validation error: a vector is not the expected length.
fn unprocessable(message: &str) -> Response {
    (
        StatusCode::UNPROCESSABLE_ENTITY,
        Json(json!({ "error": message })),
    )
        .into_response()
}

/// Call one scalar dependency at `url` with `body`, mapping its outcome to
/// either the numeric `result` (on `200`) or an early-return `Response` the
/// caller should surface verbatim:
///
/// - unreachable / non-`200`/`422` -> `503` degraded
/// - `422` -> forwarded `422` (the dependency rejected the input)
/// - `200` without a numeric `result` -> `500` malformed
async fn ask(url: &str, body: &Value, dependency: &str) -> Result<f64, Response> {
    match client::call(url, body).await {
        Err(DepError::Unreachable) => Err(degraded(dependency)),
        Ok((200, body)) => match body.get("result").and_then(Value::as_f64) {
            Some(r) => Ok(r),
            None => Err(malformed(dependency)),
        },
        Ok((422, body)) => Err(forward(422, body)),
        Ok(_) => Err(degraded(dependency)),
    }
}

/// `POST /` — the 3D cross product `a × b`.
///
/// This service owns the *control flow* but delegates every arithmetic step to
/// its float primitives, exactly as specified. For `a = [a0, a1, a2]` and
/// `b = [b0, b1, b2]`:
///
/// - `cx = floatsubtract(floatmultiply(a1, b2), floatmultiply(a2, b1))`
/// - `cy = floatsubtract(floatmultiply(a2, b0), floatmultiply(a0, b2))`
/// - `cz = floatsubtract(floatmultiply(a0, b1), floatmultiply(a1, b0))`
///
/// and `result = [cx, cy, cz]`.
///
/// Validation is not handled here for the *elements*: this service never calls
/// `srvcs-isnumber` directly. If an element is not a number a float primitive
/// rejects it with `422` and that is forwarded. Vectors must each have length
/// `3`; otherwise this service returns `422` itself.
#[utoipa::path(
    post,
    path = "/",
    request_body = EvalRequest,
    responses(
        (status = 200, body = CrossProductResponse),
        (status = 422, description = "a vector is not length 3, or a dependency rejected an element (forwarded)"),
        (status = 500, description = "a dependency returned a malformed result"),
        (status = 503, description = "a dependency is unavailable")
    )
)]
pub async fn evaluate(State(deps): State<Deps>, Json(req): Json<EvalRequest>) -> Response {
    let EvalRequest { a, b } = req;

    if a.len() != 3 {
        return unprocessable("vector a must have length 3");
    }
    if b.len() != 3 {
        return unprocessable("vector b must have length 3");
    }

    // Each component is floatsubtract(floatmultiply(..), floatmultiply(..)).
    // Index pairs for (cx, cy, cz):
    //   cx = a1*b2 - a2*b1
    //   cy = a2*b0 - a0*b2
    //   cz = a0*b1 - a1*b0
    let components = [(1, 2, 2, 1), (2, 0, 0, 2), (0, 1, 1, 0)];

    let mut result = Vec::with_capacity(3);
    for (i, j, k, l) in components {
        let lhs = match ask(
            &deps.floatmultiply_url,
            &json!({ "a": a[i], "b": b[j] }),
            "srvcs-floatmultiply",
        )
        .await
        {
            Ok(v) => v,
            Err(resp) => return resp,
        };
        let rhs = match ask(
            &deps.floatmultiply_url,
            &json!({ "a": a[k], "b": b[l] }),
            "srvcs-floatmultiply",
        )
        .await
        {
            Ok(v) => v,
            Err(resp) => return resp,
        };
        let c = match ask(
            &deps.floatsubtract_url,
            &json!({ "a": lhs, "b": rhs }),
            "srvcs-floatsubtract",
        )
        .await
        {
            Ok(v) => v,
            Err(resp) => return resp,
        };
        result.push(c);
    }

    ok(a, b, result)
}

#[derive(OpenApi)]
#[openapi(
    paths(index, evaluate),
    components(schemas(Info, EvalRequest, CrossProductResponse))
)]
pub struct ApiDoc;

/// Serve OpenAPI document
pub async fn openapi_json() -> Json<utoipa::openapi::OpenApi> {
    Json(ApiDoc::openapi())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn openapi_documents_routes() {
        let doc = ApiDoc::openapi();
        let root = doc.paths.paths.get("/").expect("path / present");
        assert!(root.get.is_some());
        assert!(root.post.is_some());
    }

    #[tokio::test]
    async fn index_reports_all_dependencies() {
        let Json(info) = index().await;
        assert_eq!(info.service, "srvcs-crossproduct");
        assert_eq!(info.concern, "vectors: 3D cross product");
        assert_eq!(
            info.depends_on,
            vec!["srvcs-floatmultiply", "srvcs-floatsubtract"]
        );
    }
}
