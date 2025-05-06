use godot::builtin::{GString, real};
use godot::init::{ExtensionLibrary, gdextension};
use godot::register::GodotConvert;

pub mod entrance;

pub mod level;

pub mod world;

pub mod player;

pub mod weapon;

pub mod bullet;

pub mod zombie;

// player
const PLAYER_MAX_HEALTH: u32 = 100;

const PLAYER_MOVE_SPEED: real = 225.0;

// weapon
const BULLET_DAMAGE: i64 = 20;

const BULLET_REPEL: f32 = 15.0;

const MAX_AMMO: i64 = 30;

const MAX_BULLET_HIT: u8 = 2;

const RELOAD_TIME: u32 = 1000;

//level
const LEVEL_RAMPAGE_TIME: u32 = 30_000;

// zombie
const ZOMBIE_DAMAGE: i64 = 10;

const ZOMBIE_PURSUIT_DISTANCE: f32 = 225.0;

const ZOMBIE_MAX_DISTANCE: f32 = 1000.0;

const ZOMBIE_MOVE_SPEED: real = 150.0;

const ZOMBIE_MAX_HEALTH: u32 = 100;

const ZOMBIE_RAMPAGE_TIME: u32 = 10_000;

#[derive(GodotConvert, Default, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Copy, Clone)]
#[godot(via = GString)]
pub enum PlayerState {
    #[default]
    Born,
    Guard,
    Run,
    Shoot,
    Reload,
    Hit,
    Dead,
}

#[derive(GodotConvert, Default, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Copy, Clone)]
#[godot(via = GString)]
pub enum ZombieState {
    #[default]
    Guard,
    Run,
    Attack,
    Dead,
}

struct Salvation;

#[gdextension]
unsafe impl ExtensionLibrary for Salvation {}
