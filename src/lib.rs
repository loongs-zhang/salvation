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

pub mod boss;

// player
const PLAYER_MAX_LIVES: u32 = 3;

const PLAYER_MAX_HEALTH: u32 = 100;

const PLAYER_LEVEL_UP_GROW_RATE: real = 1.2;

const PLAYER_LEVEL_UP_BARRIER: u32 = 2000;

const PLAYER_MOVE_SPEED: real = 225.0;

// weapon
const WEAPON_FIRE_COOLDOWN: real = 0.1;

const BULLET_DAMAGE: i64 = 20;

const BULLET_DISTANCE: real = 800.0;

const BULLET_REPEL: real = 15.0;

const BULLET_PENETRATE: real = 2.0;

const MAX_AMMO: i64 = 30;

const RELOAD_TIME: u32 = 1000;

//level
const LEVEL_GROW_RATE: real = 1.1;

const LEVEL_RAMPAGE_TIME: real = 120.0;

// zombie
const ZOMBIE_MAX_SCREEN_COUNT: u32 = 160;

const ZOMBIE_MIN_REFRESH_BATCH: u32 = 40;

const ZOMBIE_MAX_BODY_COUNT: u32 = 60;

const ZOMBIE_DAMAGE: i64 = 5;

const ZOMBIE_PURSUIT_DISTANCE: real = 225.0;

const ZOMBIE_MAX_DISTANCE: real = 1600.0;

const ZOMBIE_MOVE_SPEED: real = 150.0;

const ZOMBIE_MAX_HEALTH: u32 = 100;

const ZOMBIE_ALARM_TIME: real = 1.5;

const ZOMBIE_RAMPAGE_TIME: real = 30.0;

//boss
const BOSS_MAX_HEALTH: u32 = 3600;

const BOSS_MOVE_SPEED: real = 200.0;

const BOSS_DAMAGE: i64 = 8;

const BOSS_BUMP_DISTANCE: real = 275.0;

const BOSS_MAX_BODY_COUNT: u32 = 20;

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
pub enum PlayerUpgrade {
    #[default]
    Health,
    Penetrate,
    Damage,
    Repel,
    Lives,
    Distance,
}

#[derive(GodotConvert, Default, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Copy, Clone)]
#[godot(via = GString)]
pub enum ZombieState {
    #[default]
    Guard,
    Run,
    Rampage,
    Hit,
    Attack,
    Dead,
}

struct Salvation;

#[gdextension]
unsafe impl ExtensionLibrary for Salvation {}
