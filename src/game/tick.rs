use rand::Rng;
use uuid::Uuid;

use super::state::{
    BASE_RADIUS, BASE_SPEED, EAT_SIZE_RATIO, FOOD_GROWTH, FOOD_RADIUS, Food, GameState, WORLD_SIZE,
    distance_squared,
};

#[derive(Debug, Clone, PartialEq)]
pub struct DeathEvent {
    pub victim: Uuid,
    pub eaten_by_name: String,
}

/// Advances the simulation by one tick: movement, food consumption, then
/// player-vs-player eating. Returns the players that died this tick.
pub fn advance(state: &mut GameState, rng: &mut impl Rng) -> Vec<DeathEvent> {
    move_players(state);
    resolve_food(state, rng);
    resolve_player_collisions(state)
}

fn move_players(state: &mut GameState) {
    for player in state.players.values_mut() {
        let dx = player.target_x - player.x;
        let dy = player.target_y - player.y;
        let dist = (dx * dx + dy * dy).sqrt();

        // Bigger players move slower, same curve agar.io-likes use.
        let speed = BASE_SPEED * (BASE_RADIUS / player.radius).sqrt();

        if dist > speed {
            player.x += dx / dist * speed;
            player.y += dy / dist * speed;
        } else {
            player.x = player.target_x;
            player.y = player.target_y;
        }

        player.x = player.x.clamp(0.0, WORLD_SIZE);
        player.y = player.y.clamp(0.0, WORLD_SIZE);
    }
}

fn resolve_food(state: &mut GameState, rng: &mut impl Rng) {
    let mut eaten_food = Vec::new();

    for food in state.food.values() {
        for player in state.players.values_mut() {
            let r = player.radius + FOOD_RADIUS;
            if distance_squared(player.x, player.y, food.x, food.y) < r * r {
                player.radius += FOOD_GROWTH;
                eaten_food.push(food.id);
                break;
            }
        }
    }

    for id in eaten_food {
        state.food.remove(&id);
        let fresh = Food::random(rng);
        state.food.insert(fresh.id, fresh);
    }
}

fn resolve_player_collisions(state: &mut GameState) -> Vec<DeathEvent> {
    let snapshot: Vec<(Uuid, f32, f32, f32)> = state
        .players
        .values()
        .map(|p| (p.id, p.x, p.y, p.radius))
        .collect();

    let mut eaten_by: std::collections::HashMap<Uuid, Uuid> = std::collections::HashMap::new();

    for a in &snapshot {
        for b in &snapshot {
            if a.0 == b.0 {
                continue;
            }
            let (victim, eater) = (a, b);
            if eaten_by.contains_key(&victim.0) {
                continue;
            }
            let eater_big_enough = eater.3 >= victim.3 * EAT_SIZE_RATIO;
            if !eater_big_enough {
                continue;
            }
            // Victim's center must be inside the eater's circle to count as eaten.
            if distance_squared(victim.1, victim.2, eater.1, eater.2) < eater.3 * eater.3 {
                eaten_by.insert(victim.0, eater.0);
            }
        }
    }

    let mut events = Vec::new();
    for (&victim_id, &eater_id) in &eaten_by {
        let (victim_radius, eater_name) = {
            let victim_radius = state
                .players
                .get(&victim_id)
                .map(|p| p.radius)
                .unwrap_or(0.0);
            let eater_name = state.players.get(&eater_id).map(|p| p.name.clone());
            (victim_radius, eater_name)
        };
        let Some(eater_name) = eater_name else {
            continue;
        };
        if let Some(eater) = state.players.get_mut(&eater_id) {
            eater.radius += victim_radius * 0.5;
        }
        events.push(DeathEvent {
            victim: victim_id,
            eaten_by_name: eater_name,
        });
    }

    for event in &events {
        state.players.remove(&event.victim);
    }

    events
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::state::Player;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    fn rng() -> StdRng {
        StdRng::seed_from_u64(42)
    }

    fn player_at(id: Uuid, name: &str, x: f32, y: f32, radius: f32) -> Player {
        Player {
            id,
            name: name.to_string(),
            color: "hsl(0, 0%, 0%)".to_string(),
            x,
            y,
            target_x: x,
            target_y: y,
            radius,
        }
    }

    #[test]
    fn player_moves_toward_target() {
        let mut state = GameState::default();
        let id = Uuid::new_v4();
        let mut p = player_at(id, "a", 0.0, 0.0, BASE_RADIUS);
        p.target_x = 100.0;
        p.target_y = 0.0;
        state.players.insert(id, p);

        advance(&mut state, &mut rng());

        let moved = &state.players[&id];
        assert!(moved.x > 0.0 && moved.x < 100.0);
        assert_eq!(moved.y, 0.0);
    }

    #[test]
    fn player_stops_exactly_at_target_when_close() {
        let mut state = GameState::default();
        let id = Uuid::new_v4();
        let mut p = player_at(id, "a", 0.0, 0.0, BASE_RADIUS);
        p.target_x = 1.0;
        p.target_y = 0.0;
        state.players.insert(id, p);

        advance(&mut state, &mut rng());

        let moved = &state.players[&id];
        assert_eq!(moved.x, 1.0);
    }

    #[test]
    fn bigger_players_move_slower() {
        let mut small_state = GameState::default();
        let small_id = Uuid::new_v4();
        let mut small = player_at(small_id, "small", 0.0, 0.0, BASE_RADIUS);
        small.target_x = 1000.0;
        small_state.players.insert(small_id, small);

        let mut big_state = GameState::default();
        let big_id = Uuid::new_v4();
        let mut big = player_at(big_id, "big", 0.0, 0.0, BASE_RADIUS * 4.0);
        big.target_x = 1000.0;
        big_state.players.insert(big_id, big);

        advance(&mut small_state, &mut rng());
        advance(&mut big_state, &mut rng());

        assert!(small_state.players[&small_id].x > big_state.players[&big_id].x);
    }

    #[test]
    fn player_position_is_clamped_to_world_bounds() {
        let mut state = GameState::default();
        let id = Uuid::new_v4();
        let mut p = player_at(id, "a", 0.0, 0.0, BASE_RADIUS);
        p.target_x = -500.0;
        p.target_y = WORLD_SIZE + 500.0;
        state.players.insert(id, p);

        advance(&mut state, &mut rng());

        let moved = &state.players[&id];
        assert!(moved.x >= 0.0);
        assert!(moved.y <= WORLD_SIZE);
    }

    #[test]
    fn player_eats_overlapping_food_and_grows() {
        let mut state = GameState::default();
        let id = Uuid::new_v4();
        state
            .players
            .insert(id, player_at(id, "a", 100.0, 100.0, BASE_RADIUS));
        let food = Food {
            id: Uuid::new_v4(),
            x: 100.0,
            y: 100.0,
        };
        state.food.insert(food.id, food.clone());

        advance(&mut state, &mut rng());

        assert!(state.players[&id].radius > BASE_RADIUS);
        assert!(!state.food.contains_key(&food.id));
    }

    #[test]
    fn eaten_food_is_replaced_to_keep_count_stable() {
        let mut state = GameState::default();
        let id = Uuid::new_v4();
        state
            .players
            .insert(id, player_at(id, "a", 100.0, 100.0, BASE_RADIUS));
        let food = Food {
            id: Uuid::new_v4(),
            x: 100.0,
            y: 100.0,
        };
        state.food.insert(food.id, food);

        advance(&mut state, &mut rng());

        assert_eq!(state.food.len(), 1);
    }

    #[test]
    fn far_away_food_is_not_eaten() {
        let mut state = GameState::default();
        let id = Uuid::new_v4();
        state
            .players
            .insert(id, player_at(id, "a", 0.0, 0.0, BASE_RADIUS));
        let food = Food {
            id: Uuid::new_v4(),
            x: 2000.0,
            y: 2000.0,
        };
        state.food.insert(food.id, food.clone());

        advance(&mut state, &mut rng());

        assert_eq!(state.players[&id].radius, BASE_RADIUS);
        assert!(state.food.contains_key(&food.id));
    }

    #[test]
    fn bigger_player_eats_smaller_overlapping_player() {
        let mut state = GameState::default();
        let big_id = Uuid::new_v4();
        let small_id = Uuid::new_v4();
        state.players.insert(
            big_id,
            player_at(big_id, "big", 100.0, 100.0, BASE_RADIUS * 2.0),
        );
        state.players.insert(
            small_id,
            player_at(small_id, "small", 105.0, 100.0, BASE_RADIUS),
        );

        let events = advance(&mut state, &mut rng());

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].victim, small_id);
        assert_eq!(events[0].eaten_by_name, "big");
        assert!(!state.players.contains_key(&small_id));
        assert!(state.players[&big_id].radius > BASE_RADIUS * 2.0);
    }

    #[test]
    fn similarly_sized_players_do_not_eat_each_other() {
        let mut state = GameState::default();
        let a_id = Uuid::new_v4();
        let b_id = Uuid::new_v4();
        state
            .players
            .insert(a_id, player_at(a_id, "a", 100.0, 100.0, BASE_RADIUS));
        state
            .players
            .insert(b_id, player_at(b_id, "b", 105.0, 100.0, BASE_RADIUS));

        let events = advance(&mut state, &mut rng());

        assert!(events.is_empty());
        assert!(state.players.contains_key(&a_id));
        assert!(state.players.contains_key(&b_id));
    }

    #[test]
    fn distant_players_do_not_interact_regardless_of_size() {
        let mut state = GameState::default();
        let big_id = Uuid::new_v4();
        let small_id = Uuid::new_v4();
        state.players.insert(
            big_id,
            player_at(big_id, "big", 0.0, 0.0, BASE_RADIUS * 10.0),
        );
        state.players.insert(
            small_id,
            player_at(small_id, "small", 2000.0, 2000.0, BASE_RADIUS),
        );

        let events = advance(&mut state, &mut rng());

        assert!(events.is_empty());
        assert_eq!(state.players.len(), 2);
    }
}
