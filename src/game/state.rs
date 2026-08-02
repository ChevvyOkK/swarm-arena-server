use std::collections::HashMap;

use rand::Rng;
use uuid::Uuid;

pub const WORLD_SIZE: f32 = 3000.0;
pub const BASE_RADIUS: f32 = 20.0;
pub const FOOD_RADIUS: f32 = 5.0;
pub const FOOD_COUNT: usize = 200;
pub const FOOD_GROWTH: f32 = 1.0;
/// A player must be at least this many times bigger to eat another.
pub const EAT_SIZE_RATIO: f32 = 1.15;
/// Units per tick at BASE_RADIUS; bigger players move proportionally slower.
pub const BASE_SPEED: f32 = 6.0;

#[derive(Debug, Clone)]
pub struct Player {
    pub id: Uuid,
    pub name: String,
    pub color: String,
    pub x: f32,
    pub y: f32,
    pub target_x: f32,
    pub target_y: f32,
    pub radius: f32,
}

impl Player {
    pub fn spawn(id: Uuid, name: String, rng: &mut impl Rng) -> Self {
        let x = rng.random_range(0.0..WORLD_SIZE);
        let y = rng.random_range(0.0..WORLD_SIZE);
        Player {
            id,
            name,
            color: random_color(rng),
            x,
            y,
            target_x: x,
            target_y: y,
            radius: BASE_RADIUS,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Food {
    pub id: Uuid,
    pub x: f32,
    pub y: f32,
}

impl Food {
    pub fn random(rng: &mut impl Rng) -> Self {
        Food {
            id: Uuid::new_v4(),
            x: rng.random_range(0.0..WORLD_SIZE),
            y: rng.random_range(0.0..WORLD_SIZE),
        }
    }
}

#[derive(Debug, Default)]
pub struct GameState {
    pub players: HashMap<Uuid, Player>,
    pub food: HashMap<Uuid, Food>,
}

impl GameState {
    pub fn with_food(rng: &mut impl Rng) -> Self {
        let food = (0..FOOD_COUNT)
            .map(|_| Food::random(rng))
            .map(|f| (f.id, f))
            .collect();
        GameState {
            players: HashMap::new(),
            food,
        }
    }
}

fn random_color(rng: &mut impl Rng) -> String {
    let hue = rng.random_range(0..360);
    format!("hsl({hue}, 70%, 55%)")
}

pub fn distance_squared(ax: f32, ay: f32, bx: f32, by: f32) -> f32 {
    let dx = ax - bx;
    let dy = ay - by;
    dx * dx + dy * dy
}
