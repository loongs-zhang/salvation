use crate::player::RustPlayer;
use crate::world::RustWorld;
use crate::zombie::RustZombie;
use crate::{
    LEVEL_GROW_RATE, LEVEL_RAMPAGE_TIME, ZOMBIE_MAX_SCREEN_COUNT, ZOMBIE_MIN_REFRESH_BATCH,
};
use godot::builtin::{Array, Vector2, real};
use godot::classes::{
    AudioStreamPlayer2D, CanvasLayer, Engine, INode, InputEvent, Label, Node, PackedScene, Timer,
    VBoxContainer,
};
use godot::obj::{Base, Gd, OnReady, WithBaseField};
use godot::register::{GodotClass, godot_api};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

static RAMPAGE: AtomicBool = AtomicBool::new(false);

static KILL_COUNT: AtomicU32 = AtomicU32::new(0);

static LIVE_COUNT: AtomicU32 = AtomicU32::new(0);

#[derive(GodotClass)]
#[class(base=Node)]
pub struct RustLevel {
    #[export]
    level: u32,
    #[export]
    grow_rate: real,
    #[export]
    rampage_time: real,
    left_rampage_time: real,
    killed: AtomicU32,
    hud: OnReady<Gd<CanvasLayer>>,
    generator: OnReady<Gd<ZombieGenerator>>,
    bgm: OnReady<Gd<AudioStreamPlayer2D>>,
    rampage_bgm: OnReady<Gd<AudioStreamPlayer2D>>,
    base: Base<Node>,
}

#[godot_api]
impl INode for RustLevel {
    fn init(base: Base<Node>) -> Self {
        Self {
            level: 0,
            grow_rate: LEVEL_GROW_RATE,
            rampage_time: LEVEL_RAMPAGE_TIME,
            left_rampage_time: LEVEL_RAMPAGE_TIME,
            killed: AtomicU32::new(0),
            hud: OnReady::from_node("LevelHUD"),
            generator: OnReady::from_node("ZombieGenerator"),
            bgm: OnReady::from_node("Bgm"),
            rampage_bgm: OnReady::from_node("RampageBgm"),
            base,
        }
    }

    fn ready(&mut self) {
        self.level_up();
    }

    fn process(&mut self, delta: f64) {
        if RustWorld::is_paused() {
            return;
        }
        LIVE_COUNT.store(
            self.generator
                .bind()
                .current
                .saturating_sub(self.killed.load(Ordering::Acquire)),
            Ordering::Release,
        );
        self.left_rampage_time = (self.left_rampage_time - delta as real).max(0.0);
        self.update_rampage_hud();
        self.update_progress_hud();
        self.update_fps_hud();
        if 0.0 == self.left_rampage_time {
            RAMPAGE.store(true, Ordering::Release);
            self.play_rampage_bgm();
        } else {
            RAMPAGE.store(false, Ordering::Release);
            self.play_bgm();
        }
        if self.killed.load(Ordering::Acquire) >= self.generator.bind().current_total {
            self.level_up();
        }
    }

    fn input(&mut self, event: Gd<InputEvent>) {
        if event.is_action_pressed("k") {
            self.left_rampage_time = 0.0;
        } else if event.is_action_pressed("j") {
            //跳关
            self.level_up();
        }
    }
}

#[godot_api]
impl RustLevel {
    #[func]
    pub fn kill_confirmed(&mut self) {
        self.killed.fetch_add(1, Ordering::Release);
        KILL_COUNT.store(self.killed.load(Ordering::Acquire), Ordering::Release);
        self.update_progress_hud();
        RustPlayer::add_kill_count();
    }

    pub fn update_level_hud(&mut self) {
        let mut label = self.get_center_container().get_node_as::<Label>("Level");
        label.set_text(&format!("LEVEL {}", self.level));
        label.show();
    }

    pub fn update_rampage_hud(&mut self) {
        let mut label = self.get_center_container().get_node_as::<Label>("Rampage");
        label.set_text(&format!("ZOMBIE RAMPAGE {:.1} s", self.left_rampage_time));
        label.show();
    }

    pub fn update_progress_hud(&mut self) {
        let refreshed = self.generator.bind().current;
        let total = self.generator.bind().current_total;
        let mut label = self.get_center_container().get_node_as::<Label>("Progress");
        label.set_text(&format!(
            "PROGRESS {}/{}/{}",
            self.killed.load(Ordering::Acquire),
            refreshed,
            total
        ));
        label.show();
    }

    pub fn update_refresh_hud(&mut self) {
        let refresh_count = self.generator.bind().current_refresh_count;
        let wait_time = self.generator.get_node_as::<Timer>("Timer").get_wait_time();
        let mut label = self.get_right_container().get_node_as::<Label>("Refresh");
        label.set_text(&format!(
            "REFRESH {} ZOMBIES IN {:.0}s",
            refresh_count, wait_time
        ));
        label.show();
    }

    pub fn update_fps_hud(&mut self) {
        let mut label = self.get_right_container().get_node_as::<Label>("FPS");
        label.set_text(&format!(
            "FPS {}",
            Engine::singleton().get_frames_per_second(),
        ));
        label.show();
    }

    fn get_center_container(&mut self) -> Gd<VBoxContainer> {
        self.hud.get_node_as::<VBoxContainer>("VBoxCenter")
    }

    fn get_right_container(&mut self) -> Gd<VBoxContainer> {
        self.hud.get_node_as::<VBoxContainer>("VBoxRight")
    }

    pub fn level_up(&mut self) {
        let mut generator = self.generator.bind_mut();
        let rate = self.grow_rate.powf(self.level as f32);
        self.level += 1;
        self.killed.store(0, Ordering::Release);
        self.left_rampage_time = self.rampage_time / rate;
        generator.level_up(rate);
        drop(generator);
        self.update_level_hud();
        self.update_refresh_hud();
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
        RAMPAGE.load(Ordering::Acquire)
    }

    pub fn get_kill_count() -> u32 {
        KILL_COUNT.load(Ordering::Acquire)
    }

    pub fn get_live_count() -> u32 {
        LIVE_COUNT.load(Ordering::Acquire)
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
    refresh_barrier: u32,
    base: Base<Node>,
}

#[godot_api]
impl INode for ZombieGenerator {
    fn init(base: Base<Node>) -> Self {
        Self {
            total: 30,
            refresh_count: 3,
            refresh_time: 3.0,
            zombie_scenes: Array::new(),
            current: 0,
            current_total: 30,
            current_refresh_count: 3,
            refresh_barrier: ZOMBIE_MIN_REFRESH_BATCH,
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

    fn input(&mut self, event: Gd<InputEvent>) {
        if event.is_action_pressed("p") {
            let mut timer = self.base().get_node_as::<Timer>("Timer");
            if timer.is_stopped() {
                self.generate();
                timer.start();
            } else {
                timer.stop();
            }
        } else if event.is_action_pressed("k") {
            while self.current < self.current_total
                && self.current.saturating_sub(RustLevel::get_kill_count())
                    < ZOMBIE_MAX_SCREEN_COUNT
            {
                self.generate();
            }
        }
    }
}

#[godot_api]
impl ZombieGenerator {
    pub fn level_up(&mut self, rate: f32) {
        self.current = 0;
        self.refresh_barrier = ZOMBIE_MIN_REFRESH_BATCH;
        self.current_total = (self.total as f32 * rate) as u32;
        self.current_refresh_count = (self.refresh_count as f32 * rate) as u32;
        self.base().get_node_as::<Timer>("Timer").start();
    }

    #[func]
    pub fn generate(&mut self) {
        for _ in 0..self.current_refresh_count {
            let kill_count = RustLevel::get_kill_count();
            if 0 < kill_count
                && kill_count < self.refresh_barrier
                && self.current_refresh_count > ZOMBIE_MIN_REFRESH_BATCH
                || self.current.saturating_sub(kill_count) >= ZOMBIE_MAX_SCREEN_COUNT
            {
                break;
            }
            self.generate_zombie(RustPlayer::get_position() + RustWorld::random_position());
        }
        while RustLevel::get_kill_count() >= self.refresh_barrier {
            self.refresh_barrier += ZOMBIE_MIN_REFRESH_BATCH;
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
