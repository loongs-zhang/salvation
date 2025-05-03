use godot::builtin::GString;
use godot::init::{ExtensionLibrary, gdextension};
use godot::prelude::GodotConvert;

pub mod world;

pub mod player;

pub mod weapon;

pub mod bullet;

pub mod zombie;

const PLAYER_MAX_HEALTH: u32 = 100;

const BULLET_DAMAGE: i64 = 20;

const MAX_AMMO: i64 = 30;

const MAX_BULLET_HIT: u8 = 1;

const RELOAD_TIME: f64 = 3.0;

const ZOMBIE_DAMAGE: i64 = 10;

const ZOMBIE_MAX_HEALTH: u32 = 100;

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
