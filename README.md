# SwarmArena — server

Real-time multiplayer arena game server (agar.io-style: move around a shared
2D world, eat food to grow, eat smaller players, avoid bigger ones). Rust,
`axum`, `tokio`, one WebSocket per player, authoritative server tick at 20Hz.

Client: [swarm-arena-client](https://github.com/ChevvyOkK/swarm-arena-client)
— plain HTML5 Canvas + vanilla JS, no framework.

## Architecture

- **`game::state`** — `Player`, `Food`, `GameState`. Plain data, no I/O.
- **`game::tick`** — `advance(state, rng)`: movement, food consumption,
  player-vs-player eating, run once per tick. Pure function of state + rng,
  so it's fully unit-testable without a network or an event loop — 15 of
  this repo's tests exercise it directly.
- **`protocol`** — the JSON wire format shared with the client
  (`ClientMsg::{Join, Input}`, `ServerMsg::{Welcome, State, Died}`).
- **`main`** — one `Arc<Mutex<Hub>>` holding both the game state and each
  connected player's outgoing channel. A background task ticks the
  simulation every 50ms and pushes a state snapshot to every connection;
  each WebSocket handler just reads client input and writes it into the
  shared state.

Player-vs-player collision is O(n²) per tick — deliberately, not an
oversight. [EvoSim](https://github.com/ChevvyOkK/evosim) uses a quad-tree
for the same kind of check because it simulates thousands of agents; a
browser multiplayer game bounded by a realistic number of concurrent human
players doesn't need that complexity, and pretending it does would just be
worse code for no measurable benefit.

## A bug worth naming

`#[serde(rename_all = "camelCase")]` on an enum only renames the *variant
tags* (`Input` → `"input"`), not the fields *inside* each variant. My first
version had `ClientMsg::Input { target_x, target_y }` with that attribute
and nothing else — the client sent `targetX`/`targetY`, the server silently
failed to parse it (caught by a deliberately lenient `continue` on bad
input), and every mouse movement was dropped without a single error
anywhere. Found it by testing in a real browser rather than trusting that
green unit tests meant the wire format was right — they were testing
Rust-to-Rust round-trips, not the actual JSON.

Fixed with `rename_all_fields = "camelCase"` alongside `rename_all`, and
added tests in `protocol.rs` that assert on the literal JSON strings (both
directions) so this exact class of bug can't silently come back.

## Running locally

```bash
cargo run
# listens on :8080 by default, or $PORT if set
```

## Tests

```bash
cargo test        # 15 tests: game logic + wire-format regression tests
cargo fmt --check
cargo clippy -- -D warnings
```

## Deploying

Docker-only — Render doesn't have a native Rust runtime, and a container is
also just the right tool here regardless of platform.

```bash
docker build -t swarm-arena-server .
docker run -p 8080:8080 swarm-arena-server
```

The only configuration is `PORT` (see `.env.example`) — checked with
[envcheck](https://github.com/ChevvyOkK/envcheck) before every deploy:

```bash
envcheck --schema .env.example
```
