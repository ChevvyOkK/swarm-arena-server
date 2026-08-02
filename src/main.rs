mod game;
mod protocol;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::Router;
use axum::extract::State;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::IntoResponse;
use axum::routing::get;
use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc::{self, UnboundedSender};
use tower_http::cors::CorsLayer;
use uuid::Uuid;

use game::state::{GameState, Player, WORLD_SIZE};
use game::tick;
use protocol::{ClientMsg, ServerMsg, state_snapshot, welcome};

const TICK_MS: u64 = 50;
const MAX_NAME_LEN: usize = 16;

struct Hub {
    state: GameState,
    senders: HashMap<Uuid, UnboundedSender<Message>>,
}

type SharedHub = Arc<Mutex<Hub>>;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let mut rng = rand::rng();
    let hub: SharedHub = Arc::new(Mutex::new(Hub {
        state: GameState::with_food(&mut rng),
        senders: HashMap::new(),
    }));

    spawn_tick_loop(hub.clone());

    let app = Router::new()
        .route("/", get(health))
        .route("/ws", get(ws_handler))
        .layer(CorsLayer::permissive())
        .with_state(hub);

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);
    let listener = tokio::net::TcpListener::bind(("0.0.0.0", port))
        .await
        .unwrap();
    tracing::info!("swarm-arena-server listening on :{port}");
    axum::serve(listener, app).await.unwrap();
}

async fn health() -> &'static str {
    "swarm-arena-server: ok"
}

fn spawn_tick_loop(hub: SharedHub) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(TICK_MS));
        loop {
            interval.tick().await;
            let mut rng = rand::rng();

            let (payload, deaths, senders_snapshot) = {
                let mut guard = hub.lock().unwrap();
                let deaths = tick::advance(&mut guard.state, &mut rng);
                let payload = serde_json::to_string(&state_snapshot(&guard.state)).unwrap();
                let senders_snapshot: Vec<(Uuid, UnboundedSender<Message>)> = guard
                    .senders
                    .iter()
                    .map(|(id, tx)| (*id, tx.clone()))
                    .collect();
                (payload, deaths, senders_snapshot)
            };

            for (_, tx) in &senders_snapshot {
                let _ = tx.send(Message::Text(payload.clone().into()));
            }

            for death in deaths {
                if let Some((_, tx)) = senders_snapshot.iter().find(|(id, _)| *id == death.victim) {
                    let msg = ServerMsg::Died {
                        eaten_by: death.eaten_by_name,
                    };
                    let _ = tx.send(Message::Text(serde_json::to_string(&msg).unwrap().into()));
                }
            }
        }
    });
}

async fn ws_handler(ws: WebSocketUpgrade, State(hub): State<SharedHub>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, hub))
}

async fn handle_socket(socket: WebSocket, hub: SharedHub) {
    let id = Uuid::new_v4();
    let (mut ws_tx, mut ws_rx) = socket.split();
    let (tx, mut rx) = mpsc::unbounded_channel::<Message>();

    {
        let mut guard = hub.lock().unwrap();
        guard.senders.insert(id, tx.clone());
    }

    let forward_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if ws_tx.send(msg).await.is_err() {
                break;
            }
        }
    });

    let _ = tx.send(Message::Text(
        serde_json::to_string(&welcome(id)).unwrap().into(),
    ));

    while let Some(Ok(msg)) = ws_rx.next().await {
        match msg {
            Message::Text(text) => {
                let Ok(client_msg) = serde_json::from_str::<ClientMsg>(&text) else {
                    continue;
                };
                handle_client_msg(&hub, id, client_msg);
            }
            Message::Close(_) => break,
            _ => {}
        }
    }

    {
        let mut guard = hub.lock().unwrap();
        guard.state.players.remove(&id);
        guard.senders.remove(&id);
    }
    forward_task.abort();
}

fn handle_client_msg(hub: &SharedHub, id: Uuid, msg: ClientMsg) {
    let mut guard = hub.lock().unwrap();
    match msg {
        ClientMsg::Join { name } => {
            let mut rng = rand::rng();
            let player = Player::spawn(id, sanitize_name(&name), &mut rng);
            guard.state.players.insert(id, player);
        }
        ClientMsg::Input { target_x, target_y } => {
            if let Some(player) = guard.state.players.get_mut(&id) {
                player.target_x = target_x.clamp(0.0, WORLD_SIZE);
                player.target_y = target_y.clamp(0.0, WORLD_SIZE);
            }
        }
    }
}

fn sanitize_name(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return "Anon".to_string();
    }
    trimmed.chars().take(MAX_NAME_LEN).collect()
}
