use crate::ZOMBIE_RAMPAGE_TIME;
use crate::player::RustPlayer;
use crate::world::RustWorld;
use crate::zombie::RustZombie;
use crossbeam_utils::atomic::AtomicCell;
use godot::builtin::{Array, Vector2, real};
use godot::classes::{CanvasLayer, INode2D, Label, Node2D, PackedScene, Timer, VBoxContainer};
use godot::obj::{Base, Gd, OnReady, WithBaseField};
use godot::register::{GodotClass, godot_api};
use rand::Rng;
use rand::prelude::ThreadRng;

static RAMPAGE: AtomicCell<bool> = AtomicCell::new(false);

#[derive(GodotClass)]
#[class(base=Node2D)]
pub struct RustLevel {
    #[export]
    level: u32,
    #[export]
    rampage_time: u32,
    killed: u32,
    generator: OnReady<Gd<ZombieGenerator>>,
    base: Base<Node2D>,
}

#[godot_api]
impl INode2D for RustLevel {
    fn init(base: Base<Node2D>) -> Self {
        Self {
            level: 1,
            rampage_time: ZOMBIE_RAMPAGE_TIME,
            killed: 0,
            generator: OnReady::from_node("ZombieGenerator"),
            base,
        }
    }

    fn ready(&mut self) {
        self.update_level_hud();
        self.update_rampage_hud();
        self.update_progress_hud();
    }

    fn process(&mut self, delta: f64) {
        self.rampage_time = self.rampage_time.saturating_sub((delta * 1000.0) as u32);
        self.update_rampage_hud();
        RAMPAGE.store(self.rampage_time <= 0);
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
        label.set_text(&format!("RAMPAGE {} ms", self.rampage_time));
        label.show();
    }

    pub fn update_progress_hud(&mut self) {
        let total = self.generator.bind().total;
        let mut label = self
            .base()
            .get_node_as::<CanvasLayer>("LevelHUD")
            .get_node_as::<VBoxContainer>("VBoxContainer")
            .get_node_as::<Label>("Killed");
        label.set_text(&format!("KILLED {}/{}", self.killed, total));
        label.show();
    }

    pub fn can_rampage() -> bool {
        RAMPAGE.load()
    }
}

#[derive(GodotClass)]
#[class(base=Node2D)]
pub struct ZombieGenerator {
    #[export]
    total: u32,
    #[export]
    count: u32,
    #[export]
    refresh_time: f64,
    #[export]
    zombie_scenes: Array<Gd<PackedScene>>,
    current: u32,
    base: Base<Node2D>,
}

#[godot_api]
impl INode2D for ZombieGenerator {
    fn init(base: Base<Node2D>) -> Self {
        Self {
            total: 30,
            count: 1,
            refresh_time: 1.0,
            zombie_scenes: Array::new(),
            current: 0,
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
    #[func]
    pub fn generate(&mut self) {
        let mut rng = rand::thread_rng();
        for _ in 0..self.count {
            if self.current >= self.total {
                return;
            }
            self.generate_zombie(
                RustPlayer::get_position()
                    + Vector2::new(
                        Self::random_half_position(&mut rng),
                        Self::random_half_position(&mut rng),
                    ),
            );
            self.current += 1;
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
                }
            }
        }
    }
}
