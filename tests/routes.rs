use axum::body::Body;
use axum::extract::Json as AxumJson;
use axum::http::{Request, StatusCode};
use axum::routing::post;
use axum::{Json, Router as AxumRouter};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use srvcs_crossproduct::{api::Deps, health, router, telemetry};
use tower::ServiceExt;

const DEAD_URL: &str = "http://127.0.0.1:1";

// --- Computing mocks for every srvcs primitive this family composes over.
//
// Each reads its operands from the request body and returns the *real* answer,
// so the orchestration is genuinely exercised rather than fed a canned value.
// crossproduct only calls floatmultiply and floatsubtract; the rest are
// provided for completeness of the vector family's contract.

/// `srvcs-floatadd`: reads `{a, b}` -> `{"result": a + b}` (as f64).
#[allow(dead_code)]
async fn spawn_floatadd() -> String {
    let app = AxumRouter::new().route(
        "/",
        post(|AxumJson(body): AxumJson<Value>| async move {
            let a = body.get("a").and_then(Value::as_f64).unwrap_or(0.0);
            let b = body.get("b").and_then(Value::as_f64).unwrap_or(0.0);
            Json(json!({ "result": a + b }))
        }),
    );
    serve(app).await
}

/// `srvcs-floatsubtract`: reads `{a, b}` -> `{"result": a - b}` (as f64).
async fn spawn_floatsubtract() -> String {
    let app = AxumRouter::new().route(
        "/",
        post(|AxumJson(body): AxumJson<Value>| async move {
            let a = body.get("a").and_then(Value::as_f64).unwrap_or(0.0);
            let b = body.get("b").and_then(Value::as_f64).unwrap_or(0.0);
            Json(json!({ "result": a - b }))
        }),
    );
    serve(app).await
}

/// `srvcs-floatmultiply`: reads `{a, b}` -> `{"result": a * b}` (as f64).
async fn spawn_floatmultiply() -> String {
    let app = AxumRouter::new().route(
        "/",
        post(|AxumJson(body): AxumJson<Value>| async move {
            let a = body.get("a").and_then(Value::as_f64).unwrap_or(0.0);
            let b = body.get("b").and_then(Value::as_f64).unwrap_or(0.0);
            Json(json!({ "result": a * b }))
        }),
    );
    serve(app).await
}

/// `srvcs-floatdivide`: reads `{a, b}` -> `{"result": a / b}` (as f64).
#[allow(dead_code)]
async fn spawn_floatdivide() -> String {
    let app = AxumRouter::new().route(
        "/",
        post(|AxumJson(body): AxumJson<Value>| async move {
            let a = body.get("a").and_then(Value::as_f64).unwrap_or(0.0);
            let b = body.get("b").and_then(Value::as_f64).unwrap_or(1.0);
            Json(json!({ "result": a / b }))
        }),
    );
    serve(app).await
}

/// `srvcs-sqrt`: reads `{value}` -> `{"result": sqrt(value)}` (as f64).
#[allow(dead_code)]
async fn spawn_sqrt() -> String {
    let app = AxumRouter::new().route(
        "/",
        post(|AxumJson(body): AxumJson<Value>| async move {
            let value = body.get("value").and_then(Value::as_f64).unwrap_or(0.0);
            Json(json!({ "result": value.sqrt() }))
        }),
    );
    serve(app).await
}

/// `srvcs-acos`: reads `{value}` -> `{"result": acos(value)}` (as f64).
#[allow(dead_code)]
async fn spawn_acos() -> String {
    let app = AxumRouter::new().route(
        "/",
        post(|AxumJson(body): AxumJson<Value>| async move {
            let value = body.get("value").and_then(Value::as_f64).unwrap_or(0.0);
            Json(json!({ "result": value.acos() }))
        }),
    );
    serve(app).await
}

/// Sibling `srvcs-magnitude`: reads `{vector: [..]}` -> the real vector length.
#[allow(dead_code)]
async fn spawn_magnitude() -> String {
    let app = AxumRouter::new().route(
        "/",
        post(|AxumJson(body): AxumJson<Value>| async move {
            let v: Vec<f64> = body
                .get("vector")
                .and_then(Value::as_array)
                .map(|arr| arr.iter().filter_map(Value::as_f64).collect())
                .unwrap_or_default();
            let mag = v.iter().map(|x| x * x).sum::<f64>().sqrt();
            Json(json!({ "result": mag }))
        }),
    );
    serve(app).await
}

/// Sibling `srvcs-dotproduct`: reads `{a: [..], b: [..]}` -> the real dot product.
#[allow(dead_code)]
async fn spawn_dotproduct() -> String {
    let app = AxumRouter::new().route(
        "/",
        post(|AxumJson(body): AxumJson<Value>| async move {
            let a: Vec<f64> = body
                .get("a")
                .and_then(Value::as_array)
                .map(|arr| arr.iter().filter_map(Value::as_f64).collect())
                .unwrap_or_default();
            let b: Vec<f64> = body
                .get("b")
                .and_then(Value::as_array)
                .map(|arr| arr.iter().filter_map(Value::as_f64).collect())
                .unwrap_or_default();
            let dot: f64 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
            Json(json!({ "result": dot }))
        }),
    );
    serve(app).await
}

/// Sibling `srvcs-vectorsubtract`: reads `{a: [..], b: [..]}` -> the
/// component-wise difference array.
#[allow(dead_code)]
async fn spawn_vectorsubtract() -> String {
    let app = AxumRouter::new().route(
        "/",
        post(|AxumJson(body): AxumJson<Value>| async move {
            let a: Vec<f64> = body
                .get("a")
                .and_then(Value::as_array)
                .map(|arr| arr.iter().filter_map(Value::as_f64).collect())
                .unwrap_or_default();
            let b: Vec<f64> = body
                .get("b")
                .and_then(Value::as_array)
                .map(|arr| arr.iter().filter_map(Value::as_f64).collect())
                .unwrap_or_default();
            let diff: Vec<f64> = a.iter().zip(b.iter()).map(|(x, y)| x - y).collect();
            Json(json!({ "result": diff }))
        }),
    );
    serve(app).await
}

/// Spawn a mock returning a fixed status + body (used for error-path tests).
async fn spawn_fixed(status: StatusCode, body: Value) -> String {
    let app = AxumRouter::new().route(
        "/",
        post(move || {
            let body = body.clone();
            async move { (status, Json(body)) }
        }),
    );
    serve(app).await
}

async fn serve(app: AxumRouter) -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    format!("http://{addr}")
}

/// Both dependencies are computing mocks. Used by the correctness cases so the
/// whole pipeline is genuinely exercised.
struct Mocks {
    floatmultiply: String,
    floatsubtract: String,
}

async fn all_real() -> Mocks {
    Mocks {
        floatmultiply: spawn_floatmultiply().await,
        floatsubtract: spawn_floatsubtract().await,
    }
}

fn app(deps: Deps) -> axum::Router {
    router(telemetry::metrics_handle_for_tests(), deps)
}

fn deps_from(m: &Mocks) -> Deps {
    Deps {
        floatmultiply_url: m.floatmultiply.clone(),
        floatsubtract_url: m.floatsubtract.clone(),
    }
}

async fn crossproduct(deps: Deps, a: Value, b: Value) -> (StatusCode, Value) {
    let res = app(deps)
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/")
                .header("content-type", "application/json")
                .body(Body::from(json!({ "a": a, "b": b }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = res.status();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

fn dead_deps() -> Deps {
    Deps {
        floatmultiply_url: DEAD_URL.to_string(),
        floatsubtract_url: DEAD_URL.to_string(),
    }
}

async fn status_of(uri: &str) -> StatusCode {
    app(dead_deps())
        .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
        .await
        .unwrap()
        .status()
}

/// Extract the `result` array as a `Vec<f64>`.
fn result_vec(body: &Value) -> Vec<f64> {
    body["result"]
        .as_array()
        .expect("result is a JSON array")
        .iter()
        .map(|v| v.as_f64().expect("element is a JSON number"))
        .collect()
}

/// Assert two f64 vectors match element-wise within 1e-9.
fn assert_vec_close(got: &[f64], expected: &[f64]) {
    assert_eq!(
        got.len(),
        expected.len(),
        "length mismatch: {got:?} vs {expected:?}"
    );
    for (g, e) in got.iter().zip(expected.iter()) {
        assert!(
            (g - e).abs() < 1e-9,
            "got {g}, expected {e} (full: {got:?})"
        );
    }
}

// --- Standard endpoints. ---

#[tokio::test]
async fn healthz_ok() {
    assert_eq!(status_of("/healthz").await, StatusCode::OK);
}

#[tokio::test]
async fn readyz_reflects_state() {
    health::set_ready(true);
    assert_eq!(status_of("/readyz").await, StatusCode::OK);
}

#[tokio::test]
async fn metrics_ok() {
    assert_eq!(status_of("/metrics").await, StatusCode::OK);
}

#[tokio::test]
async fn openapi_ok() {
    assert_eq!(status_of("/openapi.json").await, StatusCode::OK);
}

#[tokio::test]
async fn generates_request_id_when_absent() {
    let res = app(dead_deps())
        .oneshot(
            Request::builder()
                .uri("/healthz")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(
        res.headers().contains_key("x-request-id"),
        "response must carry a generated x-request-id"
    );
}

#[tokio::test]
async fn index_reports_identity() {
    let res = app(dead_deps())
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let body: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["service"], "srvcs-crossproduct");
    assert_eq!(body["concern"], "vectors: 3D cross product");
    assert_eq!(
        body["depends_on"],
        json!(["srvcs-floatmultiply", "srvcs-floatsubtract"])
    );
}

// --- Correctness cases, against the computing mocks. ---

#[tokio::test]
async fn cross_x_cross_y_is_z() {
    let m = all_real().await;
    let (status, body) = crossproduct(deps_from(&m), json!([1, 0, 0]), json!([0, 1, 0])).await;
    assert_eq!(status, StatusCode::OK);
    assert_vec_close(&result_vec(&body), &[0.0, 0.0, 1.0]);
}

#[tokio::test]
async fn cross_y_cross_z_is_x() {
    let m = all_real().await;
    let (status, body) = crossproduct(deps_from(&m), json!([0, 1, 0]), json!([0, 0, 1])).await;
    assert_eq!(status, StatusCode::OK);
    assert_vec_close(&result_vec(&body), &[1.0, 0.0, 0.0]);
}

#[tokio::test]
async fn cross_parallel_is_zero() {
    let m = all_real().await;
    let (status, body) = crossproduct(deps_from(&m), json!([2, 4, 6]), json!([1, 2, 3])).await;
    assert_eq!(status, StatusCode::OK);
    assert_vec_close(&result_vec(&body), &[0.0, 0.0, 0.0]);
}

#[tokio::test]
async fn cross_general_vectors() {
    let m = all_real().await;
    // [3,-3,1] x [4,9,2] = (-3*2 - 1*9, 1*4 - 3*2, 3*9 - (-3)*4) = (-15, -2, 39)
    let (status, body) = crossproduct(deps_from(&m), json!([3, -3, 1]), json!([4, 9, 2])).await;
    assert_eq!(status, StatusCode::OK);
    assert_vec_close(&result_vec(&body), &[-15.0, -2.0, 39.0]);
}

#[tokio::test]
async fn cross_fractional_vectors() {
    let m = all_real().await;
    // [1.5, 0.0, -2.0] x [0.0, 3.0, 1.0]
    // cx = 0.0*1.0 - (-2.0)*3.0 = 6.0
    // cy = (-2.0)*0.0 - 1.5*1.0 = -1.5
    // cz = 1.5*3.0 - 0.0*0.0 = 4.5
    let (status, body) = crossproduct(
        deps_from(&m),
        json!([1.5, 0.0, -2.0]),
        json!([0.0, 3.0, 1.0]),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_vec_close(&result_vec(&body), &[6.0, -1.5, 4.5]);
}

#[tokio::test]
async fn cross_is_anticommutative() {
    let m = all_real().await;
    let (s1, b1) = crossproduct(deps_from(&m), json!([1, 0, 0]), json!([0, 1, 0])).await;
    let (s2, b2) = crossproduct(deps_from(&m), json!([0, 1, 0]), json!([1, 0, 0])).await;
    assert_eq!(s1, StatusCode::OK);
    assert_eq!(s2, StatusCode::OK);
    assert_vec_close(&result_vec(&b1), &[0.0, 0.0, 1.0]);
    assert_vec_close(&result_vec(&b2), &[0.0, 0.0, -1.0]);
}

#[tokio::test]
async fn echoes_input_vectors() {
    let m = all_real().await;
    let (status, body) = crossproduct(deps_from(&m), json!([1, 0, 0]), json!([0, 1, 0])).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["a"], json!([1, 0, 0]));
    assert_eq!(body["b"], json!([0, 1, 0]));
}

// --- Validation / error / edge cases. ---

#[tokio::test]
async fn rejects_short_vector_a() {
    let m = all_real().await;
    let (status, _) = crossproduct(deps_from(&m), json!([1, 0]), json!([0, 1, 0])).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn rejects_long_vector_b() {
    let m = all_real().await;
    let (status, _) = crossproduct(deps_from(&m), json!([1, 0, 0]), json!([0, 1, 0, 5])).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn degraded_when_all_dependencies_dead() {
    // Every dependency points at a dead port, so the first floatmultiply call
    // degrades the whole pipeline.
    let (status, body) = crossproduct(dead_deps(), json!([1, 0, 0]), json!([0, 1, 0])).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body["dependency"], "srvcs-floatmultiply");
}

#[tokio::test]
async fn degrades_when_floatsubtract_unreachable() {
    // floatmultiply is reachable so the pipeline reaches the floatsubtract
    // call, which then degrades.
    let mut deps = deps_from(&all_real().await);
    deps.floatsubtract_url = DEAD_URL.to_string();
    let (status, body) = crossproduct(deps, json!([1, 0, 0]), json!([0, 1, 0])).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body["dependency"], "srvcs-floatsubtract");
}

#[tokio::test]
async fn forwards_422_from_floatmultiply() {
    let mut deps = deps_from(&all_real().await);
    deps.floatmultiply_url = spawn_fixed(
        StatusCode::UNPROCESSABLE_ENTITY,
        json!({ "error": "value is not a number" }),
    )
    .await;
    let (status, body) = crossproduct(deps, json!(["nope", 0, 0]), json!([0, 1, 0])).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["error"], "value is not a number");
}

#[tokio::test]
async fn malformed_floatmultiply_result_is_500() {
    let mut deps = deps_from(&all_real().await);
    deps.floatmultiply_url = spawn_fixed(StatusCode::OK, json!({ "result": "not-a-number" })).await;
    let (status, body) = crossproduct(deps, json!([1, 0, 0]), json!([0, 1, 0])).await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(body["dependency"], "srvcs-floatmultiply");
}

#[tokio::test]
async fn malformed_floatsubtract_result_is_500() {
    let mut deps = deps_from(&all_real().await);
    deps.floatsubtract_url = spawn_fixed(StatusCode::OK, json!({ "result": "not-a-number" })).await;
    let (status, body) = crossproduct(deps, json!([1, 0, 0]), json!([0, 1, 0])).await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(body["dependency"], "srvcs-floatsubtract");
}

// --- Sibling computing mocks are referenced so the family contract is
// exercised and the helpers do not bit-rot. ---

#[tokio::test]
async fn sibling_mocks_compute_real_values() {
    use srvcs_crossproduct::client;

    let mag = spawn_magnitude().await;
    let (_, m) = call(&mag, &json!({ "vector": [3.0, 4.0] })).await;
    assert!((m["result"].as_f64().unwrap() - 5.0).abs() < 1e-9);

    let dot = spawn_dotproduct().await;
    let (_, d) = call(&dot, &json!({ "a": [1.0, 2.0, 3.0], "b": [4.0, 5.0, 6.0] })).await;
    assert!((d["result"].as_f64().unwrap() - 32.0).abs() < 1e-9);

    let vsub = spawn_vectorsubtract().await;
    let (_, v) = call(&vsub, &json!({ "a": [5.0, 7.0], "b": [1.0, 2.0] })).await;
    let arr: Vec<f64> = v["result"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_f64().unwrap())
        .collect();
    assert_vec_close(&arr, &[4.0, 5.0]);

    let add = spawn_floatadd().await;
    let (_, s) = call(&add, &json!({ "a": 2.0, "b": 3.0 })).await;
    assert!((s["result"].as_f64().unwrap() - 5.0).abs() < 1e-9);

    let div = spawn_floatdivide().await;
    let (_, q) = call(&div, &json!({ "a": 6.0, "b": 2.0 })).await;
    assert!((q["result"].as_f64().unwrap() - 3.0).abs() < 1e-9);

    let sqrt = spawn_sqrt().await;
    let (_, r) = call(&sqrt, &json!({ "value": 9.0 })).await;
    assert!((r["result"].as_f64().unwrap() - 3.0).abs() < 1e-9);

    let acos = spawn_acos().await;
    let (_, c) = call(&acos, &json!({ "value": 1.0 })).await;
    assert!((c["result"].as_f64().unwrap() - 0.0).abs() < 1e-9);

    async fn call(url: &str, body: &Value) -> (u16, Value) {
        match client::call(url, body).await {
            Ok((s, b)) => (s, b),
            Err(_) => panic!("mock unreachable"),
        }
    }
}
