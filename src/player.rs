use crate::weapon::RustWeapon;
use crate::world::RustWorld;
use crate::{MAX_AMMO, PLAYER_MAX_HEALTH, PLAYER_MOVE_SPEED, PlayerState};
use crossbeam_utils::atomic::AtomicCell;
use godot::builtin::{Vector2, real};
use godot::classes::input::MouseMode;
use godot::classes::node::PhysicsInterpolationMode;
use godot::classes::{
    AnimatedSprite2D, AudioStreamPlayer2D, CanvasLayer, CharacterBody2D, Control, ICanvasLayer,
    ICharacterBody2D, Input, InputEvent, Label, Node2D, TextureRect, VBoxContainer,
};
use godot::obj::{Base, Gd, OnReady, WithBaseField};
use godot::register::{GodotClass, godot_api};
use rand::Rng;

static POSITION: AtomicCell<Vector2> = AtomicCell::new(Vector2::ZERO);

static STATE: AtomicCell<PlayerState> = AtomicCell::new(PlayerState::Born);

static RELOADING: AtomicCell<f64> = AtomicCell::new(0.0);

#[derive(GodotClass)]
#[class(base=CharacterBody2D)]
pub struct RustPlayer {
    // 玩家伤害
    #[export]
    damage: i64,
    // 玩家射程
    #[export]
    distance: real,
    // 玩家穿透
    #[export]
    penetrate: u8,
    // 玩家击退
    #[export]
    repel: real,
    // 玩家最大生命值
    #[export]
    health: u32,
    // 玩家移动速度
    #[export]
    speed: real,
    current_health: u32,
    state: PlayerState,
    current_speed: real,
    animated_sprite2d: OnReady<Gd<AnimatedSprite2D>>,
    weapon: OnReady<Gd<Node2D>>,
    hud: OnReady<Gd<PlayerHUD>>,
    run_audio: OnReady<Gd<AudioStreamPlayer2D>>,
    hurt_audio1: OnReady<Gd<AudioStreamPlayer2D>>,
    hurt_audio2: OnReady<Gd<AudioStreamPlayer2D>>,
    scream_audio: OnReady<Gd<AudioStreamPlayer2D>>,
    die_audio: OnReady<Gd<AudioStreamPlayer2D>>,
    base: Base<CharacterBody2D>,
}

#[godot_api]
impl ICharacterBody2D for RustPlayer {
    fn init(base: Base<CharacterBody2D>) -> Self {
        Self {
            damage: 0,
            distance: 0.0,
            penetrate: 0,
            repel: 0.0,
            health: PLAYER_MAX_HEALTH,
            current_health: PLAYER_MAX_HEALTH,
            state: PlayerState::Born,
            speed: PLAYER_MOVE_SPEED,
            current_speed: PLAYER_MOVE_SPEED,
            animated_sprite2d: OnReady::from_node("AnimatedSprite2D"),
            weapon: OnReady::from_node("Weapon"),
            hud: OnReady::from_node("PlayerHUD"),
            run_audio: OnReady::from_node("RunAudio"),
            hurt_audio1: OnReady::from_node("HurtAudio1"),
            hurt_audio2: OnReady::from_node("HurtAudio2"),
            scream_audio: OnReady::from_node("ScreamAudio"),
            die_audio: OnReady::from_node("DieAudio"),
            base,
        }
    }

    fn ready(&mut self) {
        self.base_mut()
            .set_physics_interpolation_mode(PhysicsInterpolationMode::ON);
        let rust_weapon = self.weapon.get_node_as::<RustWeapon>("RustWeapon");
        let mut hud = self.hud.bind_mut();
        hud.update_hp_hud(self.current_health, self.health);
        hud.update_ammo_hud(rust_weapon.bind().get_ammo(), MAX_AMMO);
        hud.update_damage_hud(self.damage.saturating_add(rust_weapon.bind().get_damage()));
        hud.update_distance_hud(self.distance + rust_weapon.bind().get_distance());
        hud.update_repel_hud(self.repel + rust_weapon.bind().get_repel());
        hud.update_penetrate_hud(
            self.penetrate
                .saturating_add(rust_weapon.bind().get_penetrate()),
        );
    }

    fn physics_process(&mut self, delta: f64) {
        if PlayerState::Dead == self.state || RustWorld::is_paused() {
            return;
        }
        if PlayerState::Reload == self.state {
            let reload_cost = RELOADING.load() + delta;
            RELOADING.store(reload_cost);
            if reload_cost >= self.get_rust_weapon().bind().get_reload_time() as f64 / 1000.0 {
                self.reloaded();
            }
        }
        let player_position = self.base().get_global_position();
        POSITION.store(player_position);
        let mouse_position = self.get_mouse_position();
        self.weapon.look_at(mouse_position);
        let input = Input::singleton();
        if input.is_action_pressed("mouse_left") {
            self.shoot();
        } else if (input.is_action_pressed("shift") || input.is_action_pressed("mouse_right"))
            && (input.is_action_pressed("move_left")
                || input.is_action_pressed("move_right")
                || input.is_action_pressed("move_up")
                || input.is_action_pressed("move_down"))
        {
            self.run();
        }
        let key_direction = Vector2::new(
            input.get_axis("move_left", "move_right"),
            input.get_axis("move_up", "move_down"),
        );
        match self.state {
            PlayerState::Run => self
                .animated_sprite2d
                .look_at(player_position + key_direction),
            _ => self.animated_sprite2d.look_at(mouse_position),
        }
        let mut character_body2d = self.base.to_gd();
        if key_direction != Vector2::ZERO {
            character_body2d.set_velocity(key_direction.normalized() * self.current_speed);
        } else {
            character_body2d.set_velocity(Vector2::ZERO);
            self.guard();
        }
        character_body2d.move_and_slide();
    }

    fn input(&mut self, event: Gd<InputEvent>) {
        if RustWorld::is_paused() {
            return;
        }
        if event.is_action_pressed("r") {
            self.reload();
        } else if event.is_action_released("shift")
            || event.is_action_released("mouse_left")
            || event.is_action_released("mouse_right")
        {
            self.guard();
        }
    }
}

#[godot_api]
impl RustPlayer {
    #[func]
    pub fn on_hit(&mut self, hit_val: i64) {
        let health = self.current_health;
        self.current_health = if hit_val > 0 {
            health.saturating_sub(hit_val as u32)
        } else {
            health.saturating_add(-hit_val as u32)
        };
        self.hud
            .bind_mut()
            .update_hp_hud(self.current_health, self.health);
        if 0 != self.current_health {
            self.hit();
        } else {
            self.die();
        }
    }

    #[func]
    pub fn born(&mut self) {
        if PlayerState::Dead != self.state {
            return;
        }
        self.animated_sprite2d.play_ex().name("guard").done();
        self.current_speed = self.speed;
        self.state = PlayerState::Born;
        self.current_health = self.health;
        STATE.store(self.state);
        self.hud
            .bind_mut()
            .update_hp_hud(self.current_health, self.health);
    }

    #[func]
    pub fn guard(&mut self) {
        if PlayerState::Dead == self.state || PlayerState::Reload == self.state {
            return;
        }
        self.animated_sprite2d.play_ex().name("guard").done();
        self.current_speed = self.speed;
        self.state = PlayerState::Guard;
        STATE.store(self.state);
    }

    pub fn run(&mut self) {
        if PlayerState::Dead == self.state {
            return;
        }
        self.animated_sprite2d.play_ex().name("run").done();
        self.current_speed = self.speed * 1.5;
        self.state = PlayerState::Run;
        STATE.store(self.state);
        //打断换弹
        RELOADING.store(0.0);
        if !self.run_audio.is_playing() {
            self.run_audio.play();
        }
    }

    pub fn shoot(&mut self) {
        if PlayerState::Dead == self.state || PlayerState::Reload == self.state {
            return;
        }
        let mut rust_weapon = self.get_rust_weapon();
        if rust_weapon.bind().must_reload() {
            // 没子弹时自动装填
            self.reload();
            return;
        }
        self.animated_sprite2d.play_ex().name("guard").done();
        self.current_speed = self.speed * 0.5;
        self.state = PlayerState::Shoot;
        STATE.store(self.state);
        rust_weapon
            .bind_mut()
            .fire(self.damage, self.distance, self.penetrate, self.repel);
        self.hud
            .bind_mut()
            .update_ammo_hud(rust_weapon.bind().get_ammo(), rust_weapon.bind().get_clip());
    }

    pub fn reload(&mut self) {
        if PlayerState::Dead == self.state || !self.get_rust_weapon().bind_mut().reload() {
            return;
        }
        self.animated_sprite2d.play_ex().name("reload").done();
        self.current_speed = self.speed * 0.75;
        self.state = PlayerState::Reload;
        STATE.store(self.state);
    }

    #[func]
    pub fn reloaded(&mut self) {
        if PlayerState::Dead == self.state {
            return;
        }
        self.state = PlayerState::Guard;
        let mut rust_weapon = self.get_rust_weapon();
        let clip = rust_weapon.bind_mut().reloaded();
        self.hud
            .bind_mut()
            .update_ammo_hud(rust_weapon.bind().get_ammo(), clip);
        self.guard();
        RELOADING.store(0.0);
    }

    pub fn hit(&mut self) {
        if PlayerState::Dead == self.state {
            return;
        }
        self.animated_sprite2d.play_ex().name("hit").done();
        self.current_speed = self.speed * 0.5;
        self.state = PlayerState::Hit;
        STATE.store(self.state);
        if Self::random_bool() {
            self.hurt_audio1.play();
        } else {
            self.hurt_audio2.play();
        }
        if !self.scream_audio.is_playing() {
            self.scream_audio.play();
        }
    }

    pub fn die(&mut self) {
        if PlayerState::Dead == self.state {
            return;
        }
        self.animated_sprite2d.play_ex().name("die").done();
        self.current_speed = 0.0;
        self.state = PlayerState::Dead;
        STATE.store(self.state);
        self.die_audio.play();
        // todo 支持并检查生命数
        if let Some(mut tree) = self.base().get_tree() {
            if let Some(mut timer) = tree.create_timer(3.0) {
                timer.connect("timeout", &self.base().callable("born"));
            }
        }
    }

    pub fn get_mouse_position(&self) -> Vector2 {
        self.base().get_canvas_transform().affine_inverse()
            * self
                .base()
                .get_viewport()
                .expect("Viewport not found")
                .get_mouse_position()
    }

    pub fn get_rust_weapon(&mut self) -> Gd<RustWeapon> {
        self.weapon.get_node_as::<RustWeapon>("RustWeapon")
    }

    pub fn random_bool() -> bool {
        rand::thread_rng().gen_range(-1.0..1.0) >= 0.0
    }

    pub fn get_position() -> Vector2 {
        POSITION.load()
    }

    pub fn get_state() -> PlayerState {
        STATE.load()
    }
}

#[derive(GodotClass)]
#[class(base=CanvasLayer)]
pub struct PlayerHUD {
    cross_hair: OnReady<Gd<TextureRect>>,
    control: OnReady<Gd<Control>>,
    base: Base<CanvasLayer>,
}

#[godot_api]
impl ICanvasLayer for PlayerHUD {
    fn init(base: Base<CanvasLayer>) -> Self {
        Self {
            cross_hair: OnReady::from_node("CrossHair"),
            control: OnReady::from_node("Control"),
            base,
        }
    }

    fn ready(&mut self) {
        Input::singleton().set_mouse_mode(MouseMode::HIDDEN);
    }

    fn process(&mut self, _delta: f64) {
        let viewport = self.base().get_viewport().unwrap();
        let affine_inverse = self.base().get_transform().affine_inverse();
        let mouse_position =
            affine_inverse * viewport.get_mouse_position() - self.cross_hair.get_size() / 2.0;
        self.cross_hair.set_position(mouse_position);
    }
}

#[godot_api]
impl PlayerHUD {
    pub fn update_hp_hud(&mut self, hp: u32, max_hp: u32) {
        let mut hp_hud = self.get_container().get_node_as::<Label>("HP");
        hp_hud.set_text(&format!("HP {}/{}", hp, max_hp));
        hp_hud.show();
    }

    pub fn update_ammo_hud(&mut self, ammo: i64, clip: i64) {
        let mut ammo_hud = self.get_container().get_node_as::<Label>("Ammo");
        ammo_hud.set_text(&format!("AMMO {}/{}", ammo, clip));
        ammo_hud.show();
    }

    pub fn update_damage_hud(&mut self, damage: i64) {
        let mut damage_hud = self.get_container().get_node_as::<Label>("Damage");
        damage_hud.set_text(&format!("DAMAGE {}", damage));
        damage_hud.show();
    }

    pub fn update_distance_hud(&mut self, distance: real) {
        let mut damage_hud = self.get_container().get_node_as::<Label>("Distance");
        damage_hud.set_text(&format!("DISTANCE {:.0}", distance));
        damage_hud.show();
    }

    pub fn update_penetrate_hud(&mut self, penetrate: u8) {
        let mut penetrate_hud = self.get_container().get_node_as::<Label>("Penetrate");
        penetrate_hud.set_text(&format!("PENETRATE {}", penetrate));
        penetrate_hud.show();
    }

    pub fn update_repel_hud(&mut self, repel: real) {
        let mut repel_hud = self.get_container().get_node_as::<Label>("Repel");
        repel_hud.set_text(&format!("REPEL {}", repel));
        repel_hud.show();
    }

    fn get_container(&mut self) -> Gd<VBoxContainer> {
        self.control.get_node_as::<VBoxContainer>("VBoxContainer")
    }
}
