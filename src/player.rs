use crate::weapon::RustWeapon;
use crate::{MAX_AMMO, PLAYER_MAX_HEALTH, PlayerState};
use crossbeam_utils::atomic::AtomicCell;
use godot::builtin::{Vector2, real};
use godot::classes::{
    AnimatedSprite2D, CanvasLayer, CharacterBody2D, Control, ICanvasLayer, ICharacterBody2D, Input,
    InputEvent, Label, Node2D, VBoxContainer,
};
use godot::global::godot_print;
use godot::obj::{Base, Gd, OnReady, WithBaseField};
use godot::register::{GodotClass, godot_api};
use std::ops::Add;

static POSITION: AtomicCell<Vector2> = AtomicCell::new(Vector2::ZERO);

static STATE: AtomicCell<PlayerState> = AtomicCell::new(PlayerState::Born);

static RELOADING: AtomicCell<f64> = AtomicCell::new(0.0);

#[derive(GodotClass)]
#[class(base=CharacterBody2D)]
pub struct RustPlayer {
    #[export]
    damage: i64,
    #[export]
    max_hit_count: u8,
    #[export]
    repel: real,
    #[export]
    max_health: u32,
    health: u32,
    state: PlayerState,
    speed: real,
    animated_sprite2d: OnReady<Gd<AnimatedSprite2D>>,
    weapon: OnReady<Gd<Node2D>>,
    hud: OnReady<Gd<PlayerHUD>>,
    base: Base<CharacterBody2D>,
}

#[godot_api]
impl ICharacterBody2D for RustPlayer {
    fn init(base: Base<CharacterBody2D>) -> Self {
        Self {
            damage: 0,
            max_hit_count: 0,
            repel: 0.0,
            max_health: PLAYER_MAX_HEALTH,
            health: PLAYER_MAX_HEALTH,
            state: PlayerState::Born,
            speed: 200.0,
            animated_sprite2d: OnReady::from_node("AnimatedSprite2D"),
            weapon: OnReady::from_node("Weapon"),
            hud: OnReady::from_node("PlayerHUD"),
            base,
        }
    }

    fn ready(&mut self) {
        godot_print!(
            "Player ready with damage:{} max_hit_count:{} health:{}",
            self.damage,
            self.max_hit_count,
            self.health
        );
        let rust_weapon = self.weapon.get_node_as::<RustWeapon>("RustWeapon");
        let mut hud = self.hud.bind_mut();
        hud.update_hp_hud(self.health, self.max_health);
        hud.update_ammo_hud(rust_weapon.bind().get_ammo(), MAX_AMMO);
        hud.update_damage_hud(self.damage.saturating_add(rust_weapon.bind().get_damage()));
        hud.update_repel_hud(self.repel.add(rust_weapon.bind().get_repel()));
        hud.update_penetrate_hud(
            self.max_hit_count
                .saturating_add(rust_weapon.bind().get_max_hit_count()),
        );
    }

    fn process(&mut self, delta: f64) {
        if PlayerState::Dead == self.state {
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
        } else if input.is_action_pressed("shift") || input.is_action_pressed("mouse_right") {
            self.run();
        }
        let dir = Vector2::new(
            input.get_axis("move_left", "move_right"),
            input.get_axis("move_up", "move_down"),
        );
        match self.state {
            PlayerState::Run => self.animated_sprite2d.look_at(player_position + dir),
            _ => self.animated_sprite2d.look_at(mouse_position),
        }
        let mut character_body2d = self.base.to_gd();
        if dir != Vector2::ZERO {
            character_body2d.set_velocity(dir.normalized() * self.speed);
        } else {
            character_body2d.set_velocity(Vector2::ZERO);
            self.guard();
        }
        character_body2d.move_and_slide();
    }

    fn input(&mut self, event: Gd<InputEvent>) {
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
        let health = self.health;
        self.health = if hit_val > 0 {
            health.saturating_sub(hit_val as u32)
        } else {
            health.saturating_add(-hit_val as u32)
        };
        self.hit();
        self.hud
            .bind_mut()
            .update_hp_hud(self.health, self.max_health);
        if 0 == self.health {
            self.die();
        }
    }

    #[func]
    pub fn born(&mut self) {
        if PlayerState::Dead != self.state {
            return;
        }
        self.animated_sprite2d.play_ex().name("guard").done();
        self.speed = 200.0;
        self.state = PlayerState::Born;
        self.health = self.max_health;
        STATE.store(self.state);
        self.hud
            .bind_mut()
            .update_hp_hud(self.health, self.max_health);
    }

    #[func]
    pub fn guard(&mut self) {
        if PlayerState::Dead == self.state || PlayerState::Reload == self.state {
            return;
        }
        self.animated_sprite2d.play_ex().name("guard").done();
        self.speed = 200.0;
        self.state = PlayerState::Guard;
        STATE.store(self.state);
    }

    pub fn run(&mut self) {
        if PlayerState::Dead == self.state {
            return;
        }
        self.animated_sprite2d.play_ex().name("run").done();
        self.speed = 300.0;
        self.state = PlayerState::Run;
        STATE.store(self.state);
        //打断换弹
        RELOADING.store(0.0);
    }

    pub fn shoot(&mut self) {
        if PlayerState::Dead == self.state || PlayerState::Reload == self.state {
            return;
        }
        self.animated_sprite2d.play_ex().name("guard").done();
        self.speed = 100.0;
        self.state = PlayerState::Shoot;
        STATE.store(self.state);
        let mut rust_weapon = self.get_rust_weapon();
        rust_weapon
            .bind_mut()
            .fire(self.damage, self.max_hit_count, self.repel);
        self.hud
            .bind_mut()
            .update_ammo_hud(rust_weapon.bind().get_ammo(), rust_weapon.bind().get_clip());
    }

    pub fn reload(&mut self) {
        if PlayerState::Dead == self.state || MAX_AMMO == self.get_rust_weapon().bind().get_ammo() {
            return;
        }
        self.animated_sprite2d.play_ex().name("reload").done();
        self.speed = 125.0;
        self.state = PlayerState::Reload;
        STATE.store(self.state);
    }

    #[func]
    pub fn reloaded(&mut self) {
        self.state = PlayerState::Guard;
        let mut rust_weapon = self.get_rust_weapon();
        let clip = rust_weapon.bind_mut().reload();
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
        self.speed = 100.0;
        self.state = PlayerState::Hit;
        STATE.store(self.state);
    }

    pub fn die(&mut self) {
        if PlayerState::Dead == self.state {
            return;
        }
        self.animated_sprite2d.play_ex().name("die").done();
        self.speed = 0.0;
        self.state = PlayerState::Dead;
        STATE.store(self.state);
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
    control: OnReady<Gd<Control>>,
    base: Base<CanvasLayer>,
}

#[godot_api]
impl ICanvasLayer for PlayerHUD {
    fn init(base: Base<CanvasLayer>) -> Self {
        Self {
            control: OnReady::from_node("Control"),
            base,
        }
    }
}

#[godot_api]
impl PlayerHUD {
    pub fn update_hp_hud(&mut self, hp: u32, max_hp: u32) {
        let mut hp_hud = self
            .control
            .get_node_as::<VBoxContainer>("VBoxContainer")
            .get_node_as::<Label>("HP");
        hp_hud.set_text(&format!("HP {}/{}", hp, max_hp));
        hp_hud.show();
    }

    pub fn update_ammo_hud(&mut self, ammo: i64, clip: i64) {
        let mut ammo_hud = self
            .control
            .get_node_as::<VBoxContainer>("VBoxContainer")
            .get_node_as::<Label>("Ammo");
        ammo_hud.set_text(&format!("AMMO {}/{}", ammo, clip));
        ammo_hud.show();
    }

    pub fn update_damage_hud(&mut self, damage: i64) {
        let mut damage_hud = self
            .control
            .get_node_as::<VBoxContainer>("VBoxContainer")
            .get_node_as::<Label>("Damage");
        damage_hud.set_text(&format!("DAMAGE {}", damage));
        damage_hud.show();
    }

    pub fn update_penetrate_hud(&mut self, penetrate: u8) {
        let mut penetrate_hud = self
            .control
            .get_node_as::<VBoxContainer>("VBoxContainer")
            .get_node_as::<Label>("Penetrate");
        penetrate_hud.set_text(&format!("PENETRATE {}", penetrate));
        penetrate_hud.show();
    }

    pub fn update_repel_hud(&mut self, repel: real) {
        let mut repel_hud = self
            .control
            .get_node_as::<VBoxContainer>("VBoxContainer")
            .get_node_as::<Label>("Repel");
        repel_hud.set_text(&format!("REPEL {}", repel));
        repel_hud.show();
    }
}
