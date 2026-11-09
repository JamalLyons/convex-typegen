# advanced: an AI bookmark server

An [Axum](https://docs.rs/axum) HTTP API that saves bookmarks, with Convex as its database. You
post a URL; Convex fetches the page in the background, writes a short summary and a few tags, and
flips the bookmark from `pending` to `ready`. Clients watch that happen live over server-sent
events.

Every Convex function declares `args` **and** `returns` validators, so the Rust server only sees
generated structs, enums, and typed `Id`s. There is no manual value parsing anywhere in
[`src/`](src). The summarizer can use any OpenAI-compatible LLM, but it doesn't need one: without
LLM settings it falls back to an offline, deterministic summary, so the example runs with no API
key.

## How it fits together

```text
curl ── HTTP ──▶ Axum (src/routes.rs) ── TypedClient ──▶ Convex (convex/bookmarks.ts)
                     ▲                                       │ scheduler.runAfter(0, ...)
                     │ SSE                                   ▼
                     └──── subscription to bookmarks:list ◀── summarize:run (internal action)
                                                              fetch page → LLM or fallback
                                                              → markReady / markFailed
```

1. `POST /bookmarks` calls the `bookmarks:create` mutation. It inserts a `pending` bookmark and
   schedules the internal action `summarize:run`.
2. `summarize:run` fetches the page, extracts its title and description, summarizes it, and calls
   the internal mutation `markReady` (or `markFailed` with an error message).
3. `GET /bookmarks/stream` subscribes to `bookmarks:list` through `TypedClient::subscribe` and
   sends the list again whenever Convex reports a change.

| Concept | Where |
| --- | --- |
| String-literal enums | `status` (`pending`/`ready`/`failed`) and `summarizer` (`llm`/`fallback`) in `convex/schema.ts` |
| Nullable vs optional | `title` and `summary` are `null` until ready; `summarizer` and `error` are left out until set |
| Indexes | `by_status` on `bookmarks`; `by_tag` on `bookmarkTags` for `?tag=` |
| Typed `Id<tables::Bookmarks>` | `GET` and `DELETE /bookmarks/{id}` in `src/routes.rs` |
| Mutation, query, scheduled internal action, subscription | `convex/bookmarks.ts`, `convex/summarize.ts`, `GET /bookmarks/stream` |
| Typed `ConvexError` payload | `CreateFailure` with `with_error` in `src/model.rs` |
| Runtime errors to HTTP status codes | `src/error.rs` |

Internal functions (`summarize:run`, `markReady`, `markFailed`) don't appear in the generated
bindings, so Rust can't call them.

## Prerequisites

Rust 1.97+, Node.js 22+ with pnpm, and Docker. Everything runs on your machine; no Convex cloud
account is needed.

## 1. Start a local Convex backend

Start the open-source backend in Docker (the image is the one `just e2e` pins). Ports bind to
loopback only. `--add-host` lets the backend reach a local LLM on your machine (Docker Desktop
already provides it; Linux needs the flag).

```bash
docker run --detach --name convex-bookmarks \
  --publish 127.0.0.1:3210:3210 --publish 127.0.0.1:3211:3211 \
  --add-host host.docker.internal:host-gateway \
  --env CONVEX_CLOUD_ORIGIN=http://127.0.0.1:3210 \
  --env CONVEX_SITE_ORIGIN=http://127.0.0.1:3211 \
  --env DISABLE_BEACON=true \
  ghcr.io/convex/convex-backend@sha256:e9c084e8bd6c1f249012c672193f0988c748824f55a24cdad45f6185a8462f9a
```

Wait for the backend, then create an admin key and deploy the functions:

```bash
curl --fail --silent --retry 60 --retry-delay 1 --retry-connrefused --retry-all-errors \
  --max-time 3 http://127.0.0.1:3210/version
cd examples/advanced
export CONVEX_SELF_HOSTED_URL=http://127.0.0.1:3210
export CONVEX_SELF_HOSTED_ADMIN_KEY="$(docker exec convex-bookmarks ./generate_admin_key.sh | tail -n 1)"
pnpm install --frozen-lockfile
pnpm exec convex dev --once
```

`convex dev --once` writes `CONVEX_URL` to `.env.local`, which the server reads. Use
`pnpm exec convex dev` instead to keep redeploying as you edit `convex/`.

## 2. Choose a summarizer (optional)

The summary runs inside the Convex action, so its settings are **Convex environment variables**,
not variables of the Rust process.

**Without an LLM (default).** Leave `LLM_BASE_URL` and `LLM_MODEL` unset. The summary is the
page's meta description (or its first real paragraph), and the tags are its most frequent
keywords. Stored bookmarks show `"summarizer": "fallback"`.

**With an LLM.** Point it at any OpenAI-compatible `/chat/completions` endpoint. For a free local
model with [Ollama](https://ollama.com):

```bash
ollama pull qwen2.5:1.5b
pnpm exec convex env set LLM_BASE_URL http://host.docker.internal:11434/v1
pnpm exec convex env set LLM_MODEL qwen2.5:1.5b
```

For a hosted provider, set its base URL (for example `https://api.openai.com/v1`), a model, and
`LLM_API_KEY`, which is sent as a bearer token. Stored bookmarks show `"summarizer": "llm"`. If
the endpoint fails or replies in the wrong shape, the bookmark becomes `failed` with the reason in
`error`. Run `pnpm exec convex env remove LLM_MODEL` to go back to the fallback.

## 3. Run the server

`build.rs` generates the bindings offline into `OUT_DIR`; only the running server talks to Convex.

```bash
cargo run    # listens on http://127.0.0.1:3000; set PORT to change it
```

In a second terminal, open the live stream:

```bash
curl -N http://127.0.0.1:3000/bookmarks/stream
```

In a third, add a bookmark. The stream prints it as `pending`, then again as `ready` a moment
later:

```bash
curl -X POST http://127.0.0.1:3000/bookmarks \
  -H 'content-type: application/json' -d '{"url": "https://docs.convex.dev/home"}'
```

```text
event: bookmarks
data: [{"id":"j97...","url":"https://docs.convex.dev/home","status":"pending","title":null,"summary":null,"tags":[],"created_at":1794185039922.9}]

event: bookmarks
data: [{"id":"j97...","url":"https://docs.convex.dev/home","status":"ready","title":"Convex Docs | Convex Developer Hub","summary":"Convex is an open-source reactive database ...","tags":["convex","database","reactive","typescript"],"summarizer":"llm","created_at":1794185039922.9}]
```

The other routes:

```bash
curl http://127.0.0.1:3000/bookmarks                    # newest first
curl 'http://127.0.0.1:3000/bookmarks?status=ready'     # by_status index
curl 'http://127.0.0.1:3000/bookmarks?tag=database'     # by_tag index
curl http://127.0.0.1:3000/bookmarks/<id>
curl -X DELETE http://127.0.0.1:3000/bookmarks/<id>
curl -N 'http://127.0.0.1:3000/bookmarks/stream?status=pending'   # filters work on the stream too
```

Errors come back as `{"error": "..."}`:

| Status | When |
| --- | --- |
| 400 | The URL isn't `http(s)` (the mutation's `ConvexError`), the id isn't a bookmarks id, or `?status=` is unknown |
| 404 | The bookmark doesn't exist |
| 422 | The JSON body is missing `url` |
| 503 | The server can't reach Convex |
| 500 | Anything else, such as bindings that don't match the deployment |

## Clean up

```bash
docker rm --force convex-bookmarks
```

See the [docs site](https://ctg.jamallyons.com/) for
[typed calls and subscriptions](https://ctg.jamallyons.com/typed-client.html),
[error handling](https://ctg.jamallyons.com/errors.html), and
[what's guaranteed](https://ctg.jamallyons.com/docs/type-safety.html).
