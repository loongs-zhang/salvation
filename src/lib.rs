use godot::builtin::{Array, GString, Vector2, real};
use godot::classes::{
    AudioStream, DirAccess, DisplayServer, Input, InputEvent, InputEventAction, PackedScene,
    Texture2D,
};
use godot::init::{ExtensionLibrary, gdextension};
use godot::obj::{Gd, NewGd};
use godot::prelude::load;
use godot::register::GodotConvert;
use rand::Rng;
use std::collections::HashMap;
use std::sync::LazyLock;

// todo 增加双持刀、双持武器，双持武器时，鼠标左键开一边，鼠标右键开另一边
// todo 增加存档功能
// todo 增加僵尸死亡掉落金币，需要过去拾取
// todo 增加局外可升级的技能树
// todo 程序生成地图不清理之前生成过的
pub mod common;

pub mod entrance;

pub mod hud;

pub mod level;

pub mod world;

pub mod player;

pub mod weapon;

pub mod bullet;

pub mod grenade;

pub mod knife;

pub mod zombie;

// game info
const DEFAULT_SCREEN_SIZE: Vector2 = Vector2::new(960.0, 540.0);

// common
#[allow(clippy::declare_interior_mutable_const)]
const MESSAGE: LazyLock<Gd<PackedScene>> = LazyLock::new(|| load("res://scenes/rust_message.tscn"));

#[allow(clippy::declare_interior_mutable_const)]
const GRENADE: LazyLock<Gd<PackedScene>> =
    LazyLock::new(|| load("res://scenes/grenades/fgrenade.tscn"));

#[allow(clippy::declare_interior_mutable_const)]
const BULLET: LazyLock<Gd<PackedScene>> =
    LazyLock::new(|| load("res://scenes/bullets/rust_bullet.tscn"));

#[allow(clippy::declare_interior_mutable_const)]
const EXPLODE_AUDIOS: LazyLock<Array<Gd<AudioStream>>> = LazyLock::new(|| {
    let mut audios = Array::new();
    for i in 1..=6 {
        audios.push(&load(&format!(
            "res://asserts/player/weapons/explode{}.wav",
            i
        )));
    }
    audios
});

#[allow(clippy::declare_interior_mutable_const)]
const WEAPON_TEXTURE: LazyLock<HashMap<GString, Gd<Texture2D>>> = LazyLock::new(|| {
    const WEAPONS_DIR: &str = "res://asserts/player/weapons";
    const SUFFIX: &str = "_m.png";
    let mut map = HashMap::new();
    if let Some(mut weapons_dir) = DirAccess::open(WEAPONS_DIR) {
        for dir_name in weapons_dir.get_directories().to_vec() {
            if let Some(mut weapons_dir) = DirAccess::open(&format!("{}/{}", WEAPONS_DIR, dir_name))
            {
                for file in weapons_dir.get_files().to_vec() {
                    if file.ends_with(SUFFIX) {
                        map.insert(
                            file.replace(SUFFIX, "").to_upper(),
                            load(&format!("{}/{}/{}", WEAPONS_DIR, dir_name, file)),
                        );
                    }
                }
            }
        }
    }
    map
});

// player
const PLAYER_MAX_LIVES: u32 = 3;

const PLAYER_MAX_HEALTH: u32 = 100;

const PLAYER_LEVEL_UP_GROW_RATE: real = 1.2;

const PLAYER_LEVEL_UP_BARRIER: u32 = 2000;

const PLAYER_MOVE_SPEED: real = 225.0;

// grenade
const GRENADE_DAMAGE: i64 = 240;

const GRENADE_REPEL: real = 120.0;

const GRENADE_DISTANCE: real = 400.0;

// weapon
const WEAPON_FIRE_COOLDOWN: real = 0.1;

const BULLET_DAMAGE: i64 = 20;

const BULLET_DISTANCE: real = 800.0;

const BULLET_REPEL: real = 15.0;

const BULLET_PENETRATE: real = 2.0;

const MAX_AMMO: i32 = 30;

const RELOAD_TIME: real = 1.0;

// level
const LEVEL_GROW_RATE: real = 1.1;

const LEVEL_RAMPAGE_TIME: real = 120.0;

// zombie
const ZOMBIE_SKIP_FRAME: u128 = 3;

const ZOMBIE_MAX_SCREEN_COUNT: u32 = 160;

const ZOMBIE_REFRESH_BARRIER: u32 = 40;

const ZOMBIE_MAX_BODY_COUNT: u32 = 60;

const ZOMBIE_DAMAGE: i64 = 5;

const ZOMBIE_ALARM_DISTANCE: real = 400.0;

const ZOMBIE_PURSUIT_DISTANCE: real = 225.0;

const ZOMBIE_MAX_DISTANCE: real = 1600.0;

const ZOMBIE_MOVE_SPEED: real = 2.25;

const ZOMBIE_MAX_HEALTH: u32 = 100;

const ZOMBIE_ALARM_TIME: real = 1.5;

const ZOMBIE_RAMPAGE_TIME: real = 30.0;

// boomer
const BOOMER_MOVE_SPEED: real = 2.0;

const BOOMER_EXPLODE_COUNTDOWN: real = 1.25;

const BOOMER_DAMAGE: i64 = 50;

const BOOMER_REPEL: real = 100.0;

const BOOMER_REFRESH_BARRIER: u32 = 3;

const BOOMER_MAX_SCREEN_COUNT: u32 = 10;

// boss
const BOSS_MAX_HEALTH: u32 = 7200;

const BOSS_MOVE_SPEED: real = 2.5;

const BOSS_DAMAGE: i64 = 8;

const BOSS_BUMP_DISTANCE: real = 250.0;

const BOSS_MAX_BODY_COUNT: u32 = 20;

const BOSS_MAX_SCREEN_COUNT: u32 = 6;

const BOSS_REFRESH_BARRIER: u32 = 6;

#[derive(GodotConvert, Default, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Copy, Clone)]
#[godot(via = GString)]
pub enum PlayerState {
    #[default]
    Born,
    Guard,
    Run,
    Chop,
    Shoot,
    Reload,
    Reloading,
    Hit,
    Impact,
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

pub fn scale_rate() -> real {
    //计算缩放倍数
    let window_size = DisplayServer::singleton()
        .screen_get_size_ex()
        .screen(DisplayServer::SCREEN_PRIMARY)
        .done();
    (window_size.x as real / DEFAULT_SCREEN_SIZE.x)
        .min(window_size.y as real / DEFAULT_SCREEN_SIZE.y)
        .max(1.0)
}

pub fn random_bool() -> bool {
    rand::thread_rng().gen_range(-1.0..=1.0) >= 0.0
}

pub fn random_direction() -> Vector2 {
    let mut rng = rand::thread_rng();
    Vector2::new(rng.gen_range(-1.0..=1.0), rng.gen_range(-1.0..=1.0)).normalized()
}

pub fn random_position(from: real, to: real) -> Vector2 {
    Vector2::new(
        random_half_position(from, to),
        random_half_position(from, to),
    )
}

fn random_half_position(from: real, to: real) -> real {
    let mut rng = rand::thread_rng();
    if rng.gen_range(-1.0..=1.0) >= 0.0 {
        rng.gen_range(from..to)
    } else {
        rng.gen_range(-to..=-from)
    }
}

pub fn kill_all_zombies() {
    //在这一帧清理所有僵尸
    let mut event = InputEventAction::new_gd();
    event.set_action("k");
    event.set_pressed(true);
    Input::singleton().parse_input_event(&event.upcast::<InputEvent>());
}
