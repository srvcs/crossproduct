# srvcs-crossproduct

The 3D cross-product orchestrator of the srvcs.cloud distributed standard
library.

Its single concern: **`a × b`** for two 3D vectors. It does no arithmetic of its
own. For `a = [a0, a1, a2]` and `b = [b0, b1, b2]` it composes the float
primitives into each component:

- `cx = floatsubtract(floatmultiply(a1, b2), floatmultiply(a2, b1))`
- `cy = floatsubtract(floatmultiply(a2, b0), floatmultiply(a0, b2))`
- `cz = floatsubtract(floatmultiply(a0, b1), floatmultiply(a1, b0))`

and returns `result = [cx, cy, cz]`.

Element validation propagates from the leaves: if an element is not a valid
number, a float primitive rejects it with `422` and this service forwards that
rejection unchanged. Each vector must have length `3`; a mismatch is rejected
with `422` here.

## API

| Method | Path | Purpose |
| --- | --- | --- |
| `GET` | `/` | Service identity, concern, and dependency list |
| `POST` | `/` | Compute `a × b` |
| `GET` | `/healthz` `/readyz` `/metrics` `/openapi.json` | srvcs service standard surface |

```sh
curl -s -X POST localhost:8080/ -H 'content-type: application/json' \
  -d '{"a": [1, 0, 0], "b": [0, 1, 0]}'
# {"a":[1,0,0],"b":[0,1,0],"result":[0.0,0.0,1.0]}
```

Responses:

- `200 {"a": [...], "b": [...], "result": [cx, cy, cz]}` — evaluated.
- `422` — a vector is not length 3, or an element is invalid (forwarded from a
  leaf dependency).
- `500` — a dependency returned a malformed result.
- `503` — a dependency is unavailable.

## Dependencies

- [`srvcs-floatmultiply`](https://github.com/srvcs/floatmultiply)
- [`srvcs-floatsubtract`](https://github.com/srvcs/floatsubtract)

A single request fans out across the dependency graph: each of the three result
components requires two `srvcs-floatmultiply` calls and one
`srvcs-floatsubtract` call.

## Configuration

| Variable | Default | Purpose |
| --- | --- | --- |
| `SRVCS_BIND_ADDR` | `0.0.0.0:8080` | Bind address |
| `SRVCS_FLOATMULTIPLY_URL` | `http://127.0.0.1:8091` | Base URL of `srvcs-floatmultiply` |
| `SRVCS_FLOATSUBTRACT_URL` | `http://127.0.0.1:8090` | Base URL of `srvcs-floatsubtract` |
| `SRVCS_ENV` | `development` | Environment label for logs |
| `RUST_LOG` | `info,tower_http=info` | Tracing filter |

## Local checks

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

Orchestration tests stand up mock `srvcs-floatmultiply` and
`srvcs-floatsubtract` services in-process (computing the real products and
differences), covering the happy path against `1e-9` tolerance, a degraded
dependency (`503`), a forwarded `422`, and length-mismatch validation. See
[`srvcs/platform`](https://github.com/srvcs/platform) for the shared standard.

> Note: the `cargoHash` in `flake.nix` is inherited from the template and must be
> refreshed with a `nix build` before the Nix gates pass.
