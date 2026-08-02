use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::game::state::{Food, GameState, Player, WORLD_SIZE};

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum ClientMsg {
    /// Sent on first connect, and again after death to respawn.
    Join {
        name: String,
    },
    Input {
        target_x: f32,
        target_y: f32,
    },
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum ServerMsg {
    Welcome {
        id: Uuid,
        world_size: f32,
    },
    State {
        players: Vec<PlayerView>,
        food: Vec<FoodView>,
    },
    Died {
        eaten_by: String,
    },
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerView {
    pub id: Uuid,
    pub name: String,
    pub color: String,
    pub x: f32,
    pub y: f32,
    pub radius: f32,
}

impl From<&Player> for PlayerView {
    fn from(p: &Player) -> Self {
        PlayerView {
            id: p.id,
            name: p.name.clone(),
            color: p.color.clone(),
            x: p.x,
            y: p.y,
            radius: p.radius,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FoodView {
    pub x: f32,
    pub y: f32,
}

impl From<&Food> for FoodView {
    fn from(f: &Food) -> Self {
        FoodView { x: f.x, y: f.y }
    }
}

pub fn welcome(id: Uuid) -> ServerMsg {
    ServerMsg::Welcome {
        id,
        world_size: WORLD_SIZE,
    }
}

pub fn state_snapshot(state: &GameState) -> ServerMsg {
    ServerMsg::State {
        players: state.players.values().map(PlayerView::from).collect(),
        food: state.food.values().map(FoodView::from).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // These check the actual JSON wire format, not just that the Rust types
    // round-trip through themselves - `rename_all` on an enum only renames
    // variant tags, not the fields inside each variant, which silently broke
    // every ClientMsg::Input message (target_x/target_y instead of the
    // targetX/targetY the client actually sends) until `rename_all_fields`
    // was added alongside it.

    #[test]
    fn client_input_msg_parses_camel_case_fields() {
        let json = r#"{"type":"input","targetX":10.0,"targetY":20.0}"#;
        let msg: ClientMsg = serde_json::from_str(json).unwrap();
        match msg {
            ClientMsg::Input { target_x, target_y } => {
                assert_eq!(target_x, 10.0);
                assert_eq!(target_y, 20.0);
            }
            _ => panic!("expected Input variant"),
        }
    }

    #[test]
    fn client_join_msg_parses() {
        let json = r#"{"type":"join","name":"Alice"}"#;
        let msg: ClientMsg = serde_json::from_str(json).unwrap();
        match msg {
            ClientMsg::Join { name } => assert_eq!(name, "Alice"),
            _ => panic!("expected Join variant"),
        }
    }

    #[test]
    fn server_welcome_msg_serializes_camel_case_fields() {
        let json = serde_json::to_string(&welcome(Uuid::nil())).unwrap();
        assert!(json.contains("\"worldSize\""), "got: {json}");
        assert!(!json.contains("world_size"), "got: {json}");
    }

    #[test]
    fn server_died_msg_serializes_camel_case_fields() {
        let json = serde_json::to_string(&ServerMsg::Died { eaten_by: "Bob".into() }).unwrap();
        assert!(json.contains("\"eatenBy\":\"Bob\""), "got: {json}");
    }

    #[test]
    fn server_state_msg_round_trips_through_json() {
        let mut state = GameState::default();
        let mut rng = rand::rng();
        let player = Player::spawn(Uuid::nil(), "Alice".into(), &mut rng);
        state.players.insert(player.id, player);

        let json = serde_json::to_string(&state_snapshot(&state)).unwrap();
        assert!(json.contains("\"type\":\"state\""), "got: {json}");
        assert!(json.contains("\"Alice\""), "got: {json}");
    }
}
