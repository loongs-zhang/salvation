use crate::level::RustLevel;
use crate::player::RustPlayer;
use crate::world::RustWorld;
use crate::{ZOMBIE_MAX_SCREEN_COUNT, ZOMBIE_REFRESH_BARRIER, random_degree};
use godot::builtin::Array;
use godot::classes::{INode2D, InputEvent, Node, Node2D, PackedScene, Timer};
use godot::global::godot_error;
use godot::meta::ToGodot;
use godot::obj::{Base, Gd, OnReady, WithBaseField};
use godot::register::{GodotClass, godot_api};
use std::sync::atomic::{AtomicU32, Ordering};

pub mod save;

#[derive(GodotClass, Debug)]
#[class(base=Node2D)]
pub struct ZombieGenerator {
    #[doc = "是否立刻刷新一波"]
    #[export]
    immediate: bool,
    #[doc = "当前等级已击杀的僵尸总数"]
    pub(crate) killed: AtomicU32,
    #[doc = "刷新的僵尸总数"]
    #[export]
    total: u32,
    #[doc = "每次刷新的僵尸数"]
    #[export]
    refresh_count: u32,
    #[doc = "积攒到多少僵尸再一波刷新"]
    #[export]
    pub(crate) refresh_barrier: u32,
    #[doc = "每隔多少秒刷新一波"]
    #[export]
    refresh_time: f64,
    #[doc = "最大僵尸同屏数量"]
    #[export]
    max_screen_count: u32,
    #[doc = "僵尸实体场景列表"]
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
            killed: AtomicU32::new(0),
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
                self.start_timer();
            } else {
                self.timer.stop();
            }
            self.update_refresh_hud();
        } else if event.is_action_pressed("l") {
            RustPlayer::reset_last_score_update();
            if self.current.saturating_sub(self.get_killed()) < self.max_screen_count {
                self.timer.set_wait_time(0.2);
                self.timer.start();
                self.update_refresh_hud();
                self.generate();
            }
        }
    }
}

#[godot_api]
impl ZombieGenerator {
    pub fn level_up(
        &mut self,
        jump: bool,
        rate: f32,
        refresh_barrier: u32,
        max_screen_count: u32,
        refresh_time: f64,
    ) {
        self.current = 0;
        self.killed.store(0, Ordering::Release);
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
        self.start_timer();
    }

    #[func]
    pub fn start_timer(&mut self) {
        if self.current >= self.current_total {
            return;
        }
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
            let kill_count = self.get_killed();
            let live_count = self.current.saturating_sub(kill_count);
            if 0 < kill_count
                && kill_count < self.current_refresh_barrier
                && self.current_refresh_barrier < self.current_total
                && 0 < live_count
                && live_count < self.refresh_barrier
                || live_count >= self.max_screen_count
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
        while self.get_killed() >= self.current_refresh_barrier {
            self.current_refresh_barrier += self.refresh_barrier;
        }
    }

    fn update_refresh_hud(&self) {
        if let Some(mut level) = RustLevel::get() {
            level.call_deferred("update_refresh_hud", &[]);
        }
    }

    #[func]
    pub fn kill_confirmed(&mut self) {
        self.killed.fetch_add(1, Ordering::Release);
        if let Some(mut level) = RustLevel::get() {
            level.call_deferred("update_progress_hud", &[]);
        }
        RustPlayer::get().call_deferred("add_kill_count", &[]);
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
                zombie.set_global_rotation_degrees(random_degree());
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

    pub fn get_killed(&self) -> u32 {
        self.killed.load(Ordering::Acquire)
    }
}
