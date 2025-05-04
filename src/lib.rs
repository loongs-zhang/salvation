use godot::builtin::GString;
use godot::init::{ExtensionLibrary, gdextension};
use godot::register::GodotConvert;

pub mod entrance;

pub mod world;

pub mod player;

pub mod weapon;

pub mod bullet;

pub mod zombie;

// player
const PLAYER_MAX_HEALTH: u32 = 100;

// weapon
const BULLET_DAMAGE: i64 = 2;

const BULLET_REPEL: f32 = 15.0;

const MAX_AMMO: i64 = 30;

const MAX_BULLET_HIT: u8 = 2;

const RELOAD_TIME: u32 = 2000;

// zombie
const ZOMBIE_DAMAGE: i64 = 10;

const ZOMBIE_MAX_HEALTH: u32 = 100;

const ZOMBIE_RAMPAGE_TIME: u32 = 60_000;

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
