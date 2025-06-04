use crate::level::RustLevel;
use crate::player::RustPlayer;
use crate::world::RustWorld;
use crate::{ZOMBIE_MAX_SCREEN_COUNT, ZOMBIE_REFRESH_BARRIER};
use godot::builtin::Array;
use godot::classes::{INode2D, InputEvent, Node, Node2D, PackedScene, Timer};
use godot::global::godot_error;
use godot::meta::ToGodot;
use godot::obj::{Base, Gd, OnReady, WithBaseField};
use godot::register::{GodotClass, godot_api};

pub mod save;

#[derive(GodotClass, Debug)]
#[class(base=Node2D)]
pub struct ZombieGenerator {
    #[export]
    immediate: bool,
    #[export]
    boss: bool,
    #[export]
    total: u32,
    #[export]
    refresh_count: u32,
    #[export]
    pub(crate) refresh_barrier: u32,
    #[export]
    refresh_time: f64,
    #[export]
    max_screen_count: u32,
    #[export]
    zombie_scenes: Array<Gd<PackedScene>>,
    pub(crate) current_total: u32,
    pub(crate) current_refresh_count: u32,
    pub(crate) current: u32,
    current_refresh_barrier: u32,
    timer: OnReady<Gd<Timer>>,
    base: Base<Node2D>,
}

#[godot_api]
impl INode2D for ZombieGenerator {
    fn init(base: Base<Node2D>) -> Self {
        Self {
            immediate: false,
            boss: false,
            total: 30,
            refresh_count: 3,
            refresh_barrier: ZOMBIE_REFRESH_BARRIER,
            refresh_time: 3.0,
            max_screen_count: ZOMBIE_MAX_SCREEN_COUNT,
            zombie_scenes: Array::new(),
            current: 0,
            current_total: 30,
            current_refresh_count: 3,
            current_refresh_barrier: ZOMBIE_REFRESH_BARRIER,
            timer: OnReady::from_node("Timer"),
            base,
        }
    }

    fn ready(&mut self) {
        let callable = self.base().callable("generate");
        self.timer.connect("timeout", &callable);
        self.timer.set_wait_time(self.refresh_time);
        self.timer.set_one_shot(false);
        self.timer.set_autostart(true);
        self.timer.start();
    }

    fn input(&mut self, event: Gd<InputEvent>) {
        if event.is_action_pressed("esc") {
            RustPlayer::reset_last_score_update();
            if self.timer.is_stopped() {
                self.timer.start();
            } else {
                self.timer.stop();
            }
            self.update_refresh_hud();
        } else if event.is_action_pressed("l") {
            RustPlayer::reset_last_score_update();
            if self.current < self.current_total
                && self.current.saturating_sub(self.get_kill_count()) < self.max_screen_count
            {
                self.timer.set_wait_time(0.2);
                self.timer.start();
                self.update_refresh_hud();
            }
        }
    }
}

#[godot_api]
impl ZombieGenerator {
    pub fn level_up(
        &mut self,
        jump: bool,
        boss: bool,
        rate: f32,
        refresh_barrier: u32,
        max_screen_count: u32,
        refresh_time: f64,
    ) {
        self.current = 0;
        self.boss = boss;
        self.refresh_barrier = refresh_barrier;
        self.current_refresh_barrier = refresh_barrier;
        self.max_screen_count = max_screen_count;
        self.current_total = (self.total as f32 * rate) as u32;
        self.current_refresh_count =
            ((self.refresh_count as f32 * rate) as u32).min(refresh_barrier);
        self.timer.set_wait_time(refresh_time);
        if !RustWorld::is_paused() {
            self.timer.start();
        }
        if !jump && self.immediate {
            self.generate();
        }
    }

    pub fn refresh_timer(&mut self, refresh_time: f64) {
        let current_wait_time = self.timer.get_wait_time();
        if 0.2 == current_wait_time || current_wait_time == refresh_time && !self.timer.is_stopped()
        {
            return;
        }
        self.timer.set_wait_time(refresh_time);
        self.timer.start();
        self.update_refresh_hud();
    }

    #[func]
    pub fn start_timer(&mut self) {
        self.timer.start();
        self.update_refresh_hud();
    }

    #[func]
    pub fn stop_timer(&mut self) {
        self.timer.stop();
        self.update_refresh_hud();
    }

    #[func]
    pub fn generate(&mut self) {
        for _ in 0..self.current_refresh_count {
            let kill_count = self.get_kill_count();
            if 0 < kill_count && kill_count < self.current_refresh_barrier
                || self.current.saturating_sub(kill_count) >= self.max_screen_count
            {
                break;
            }
            if self.current >= self.current_total {
                self.stop_timer();
                break;
            }
            self.generate_zombie();
            self.current += 1;
        }
        while self.get_kill_count() >= self.current_refresh_barrier {
            self.current_refresh_barrier += self.refresh_barrier;
        }
    }

    fn update_refresh_hud(&self) {
        self.base()
            .get_parent()
            .unwrap()
            .call_deferred("update_refresh_hud", &[]);
    }

    fn get_kill_count(&self) -> u32 {
        if self.boss {
            RustLevel::get_kill_boss_count()
        } else {
            RustLevel::get_kill_count()
        }
    }

    pub fn generate_zombie(&self) {
        if !self.base().is_visible() {
            return;
        }
        let mut zombies = Vec::new();
        for zombie_scene in self.zombie_scenes.iter_shared() {
            if let Some(mut zombie) = zombie_scene.try_instantiate_as::<Node2D>() {
                zombie
                    .set_global_position(RustPlayer::get_position() + RustWorld::random_position());
                zombies.push(zombie.to_variant());
            }
        }
        if let Some(tree) = self.base().get_tree() {
            if let Some(root) = tree.get_root() {
                root.get_node_as::<Node>("RustWorld")
                    .call_deferred("add_child", &zombies);
                return;
            }
        }
        godot_error!("Failed to instantiate zombie scene");
    }
}
