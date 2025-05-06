use crate::LEVEL_RAMPAGE_TIME;
use crate::player::RustPlayer;
use crate::world::RustWorld;
use crate::zombie::RustZombie;
use crossbeam_utils::atomic::AtomicCell;
use godot::builtin::{Array, Vector2, real};
use godot::classes::{
    AudioStreamPlayer2D, CanvasLayer, Engine, INode, Label, Node, PackedScene, Timer, VBoxContainer,
};
use godot::obj::{Base, Gd, OnReady, WithBaseField};
use godot::register::{GodotClass, godot_api};
use rand::Rng;
use rand::prelude::ThreadRng;

static RAMPAGE: AtomicCell<bool> = AtomicCell::new(false);

#[derive(GodotClass)]
#[class(base=Node)]
pub struct RustLevel {
    #[export]
    level: u32,
    #[export]
    grow_rate: f32,
    #[export]
    rampage_time: u32,
    left_rampage_time: u32,
    killed: u32,
    generator: OnReady<Gd<ZombieGenerator>>,
    bgm: OnReady<Gd<AudioStreamPlayer2D>>,
    rampage_bgm: OnReady<Gd<AudioStreamPlayer2D>>,
    base: Base<Node>,
}

#[godot_api]
impl INode for RustLevel {
    fn init(base: Base<Node>) -> Self {
        Self {
            level: 1,
            grow_rate: 1.1,
            rampage_time: LEVEL_RAMPAGE_TIME,
            left_rampage_time: LEVEL_RAMPAGE_TIME,
            killed: 0,
            generator: OnReady::from_node("ZombieGenerator"),
            bgm: OnReady::from_node("Bgm"),
            rampage_bgm: OnReady::from_node("RampageBgm"),
            base,
        }
    }

    fn ready(&mut self) {
        self.update_level_hud();
        self.play_bgm();
    }

    fn process(&mut self, delta: f64) {
        self.left_rampage_time = self
            .left_rampage_time
            .saturating_sub((delta * 1000.0) as u32);
        self.update_rampage_hud();
        self.update_progress_hud();
        self.update_fps_hud();
        if self.left_rampage_time <= 0 {
            RAMPAGE.store(true);
            self.play_rampage_bgm();
        } else {
            RAMPAGE.store(false);
            self.play_bgm();
        }
        self.level_up();
    }
}

#[godot_api]
impl RustLevel {
    #[func]
    pub fn kill_confirmed(&mut self) {
        self.killed += 1;
        self.update_progress_hud();
    }

    pub fn update_level_hud(&mut self) {
        let mut label = self
            .base()
            .get_node_as::<CanvasLayer>("LevelHUD")
            .get_node_as::<VBoxContainer>("VBoxContainer")
            .get_node_as::<Label>("Level");
        label.set_text(&format!("LEVEL {}", self.level));
        label.show();
    }

    pub fn update_rampage_hud(&mut self) {
        let mut label = self
            .base()
            .get_node_as::<CanvasLayer>("LevelHUD")
            .get_node_as::<VBoxContainer>("VBoxContainer")
            .get_node_as::<Label>("Rampage");
        label.set_text(&format!("RAMPAGE {} ms", self.left_rampage_time));
        label.show();
    }

    pub fn update_progress_hud(&mut self) {
        let refreshed = self.generator.bind().current;
        let total = self.generator.bind().current_total;
        let mut label = self
            .base()
            .get_node_as::<CanvasLayer>("LevelHUD")
            .get_node_as::<VBoxContainer>("VBoxContainer")
            .get_node_as::<Label>("Killed");
        label.set_text(&format!("KILLED {}/{}/{}", self.killed, refreshed, total));
        label.show();
    }

    pub fn update_fps_hud(&mut self) {
        let engine = Engine::singleton();
        let mut label = self
            .base()
            .get_node_as::<CanvasLayer>("LevelHUD")
            .get_node_as::<Label>("FPS");
        label.set_text(&format!("FPS {}", engine.get_frames_per_second(),));
        label.show();
    }

    pub fn level_up(&mut self) {
        let mut generator = self.generator.bind_mut();
        if self.killed < generator.total {
            return;
        }
        let rate = self.grow_rate.powf(self.level as f32);
        self.level += 1;
        self.killed = 0;
        self.left_rampage_time = (self.rampage_time as f32 / rate) as u32;
        generator.level_up(rate);
        drop(generator);
        self.update_level_hud();
        self.update_progress_hud();
    }

    pub fn play_bgm(&mut self) {
        if self.bgm.is_playing() {
            return;
        }
        self.rampage_bgm.stop();
        self.bgm.play();
    }

    pub fn play_rampage_bgm(&mut self) {
        if self.rampage_bgm.is_playing() {
            return;
        }
        self.bgm.stop();
        self.rampage_bgm.play();
    }

    pub fn is_rampage() -> bool {
        RAMPAGE.load()
    }
}

#[derive(GodotClass)]
#[class(base=Node)]
pub struct ZombieGenerator {
    #[export]
    total: u32,
    #[export]
    refresh_count: u32,
    #[export]
    refresh_time: f64,
    #[export]
    zombie_scenes: Array<Gd<PackedScene>>,
    current_total: u32,
    current_refresh_count: u32,
    current: u32,
    base: Base<Node>,
}

#[godot_api]
impl INode for ZombieGenerator {
    fn init(base: Base<Node>) -> Self {
        Self {
            total: 30,
            refresh_count: 3,
            refresh_time: 1.0,
            zombie_scenes: Array::new(),
            current: 0,
            current_total: 30,
            current_refresh_count: 3,
            base,
        }
    }

    fn ready(&mut self) {
        let mut timer = self.base().get_node_as::<Timer>("Timer");
        timer.connect("timeout", &self.base_mut().callable("generate"));
        timer.set_wait_time(self.refresh_time);
        timer.set_one_shot(false);
        timer.set_autostart(true);
        timer.start();
    }
}

#[godot_api]
impl ZombieGenerator {
    pub fn level_up(&mut self, rate: f32) {
        if self.current_total <= self.current {
            self.current = 0;
            self.current_total = ((self.total as f32 * rate) as u32).min(120);
            self.current_refresh_count = ((self.refresh_count as f32 * rate) as u32).min(120);
            self.base().get_node_as::<Timer>("Timer").start();
        }
    }

    #[func]
    pub fn generate(&mut self) {
        let mut rng = rand::thread_rng();
        for _ in 0..self.current_refresh_count {
            self.generate_zombie(
                RustPlayer::get_position()
                    + Vector2::new(
                        Self::random_half_position(&mut rng),
                        Self::random_half_position(&mut rng),
                    ),
            );
        }
    }

    pub fn random_half_position(rng: &mut ThreadRng) -> real {
        if rng.gen_range(-1.0..1.0) >= 0.0 {
            rng.gen_range(250.0..500.0)
        } else {
            rng.gen_range(-500.0..-250.0)
        }
    }

    pub fn generate_zombie(&mut self, position: Vector2) {
        if self.current >= self.current_total {
            self.base().get_node_as::<Timer>("Timer").stop();
            return;
        }
        let mut zombies = Vec::new();
        for zombie_scene in self.zombie_scenes.iter_shared() {
            if let Some(mut zombie) = zombie_scene.try_instantiate_as::<RustZombie>() {
                zombie.set_global_position(position);
                zombies.push(zombie);
            }
        }
        if let Some(tree) = self.base().get_tree() {
            if let Some(root) = tree.get_root() {
                let mut world = root.get_node_as::<RustWorld>("RustWorld");
                for zombie in zombies {
                    world.add_child(&zombie);
                    self.current += 1;
                }
            }
        }
    }
}
