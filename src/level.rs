use crate::player::RustPlayer;
use crate::world::RustWorld;
use crate::{
    BOSS_MAX_SCREEN_COUNT, BOSS_REFRESH_BARRIER, LEVEL_GROW_RATE, LEVEL_RAMPAGE_TIME,
    ZOMBIE_MAX_SCREEN_COUNT, ZOMBIE_REFRESH_BARRIER,
};
use godot::builtin::{Array, real};
use godot::classes::{
    AudioStreamPlayer2D, CanvasLayer, Engine, INode, InputEvent, Label, Node, Node2D, PackedScene,
    Timer, VBoxContainer,
};
use godot::global::godot_print;
use godot::obj::{Base, Gd, OnReady, WithBaseField};
use godot::prelude::ToGodot;
use godot::register::{GodotClass, godot_api};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static RAMPAGE: AtomicBool = AtomicBool::new(false);

static KILL_COUNT: AtomicU32 = AtomicU32::new(0);

static KILL_BOSS_COUNT: AtomicU32 = AtomicU32::new(0);

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
    zombie_killed: AtomicU32,
    boss_killed: AtomicU32,
    hud: OnReady<Gd<CanvasLayer>>,
    zombie_generator: OnReady<Gd<ZombieGenerator>>,
    boss_generator: OnReady<Gd<ZombieGenerator>>,
    bgm: OnReady<Gd<AudioStreamPlayer2D>>,
    rampage_bgm: OnReady<Gd<AudioStreamPlayer2D>>,
    boss_bgm: OnReady<Gd<AudioStreamPlayer2D>>,
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
            zombie_killed: AtomicU32::new(0),
            boss_killed: AtomicU32::new(0),
            hud: OnReady::from_node("LevelHUD"),
            zombie_generator: OnReady::from_node("ZombieGenerator"),
            boss_generator: OnReady::from_node("BossGenerator"),
            bgm: OnReady::from_node("Bgm"),
            rampage_bgm: OnReady::from_node("RampageBgm"),
            boss_bgm: OnReady::from_node("BossBgm"),
            base,
        }
    }

    fn process(&mut self, delta: f64) {
        if RustWorld::is_paused() {
            return;
        }
        let zombie_killed = self.zombie_killed.load(Ordering::Acquire);
        let boss_killed = self.boss_killed.load(Ordering::Acquire);
        let killed = zombie_killed.saturating_add(boss_killed);
        let zombie_generator = self.zombie_generator.bind();
        let boss_generator = self.boss_generator.bind();
        let zombie_current = zombie_generator.current;
        let boss_current = boss_generator.current;
        let zombie_total = zombie_generator.current_total;
        let boss_total = boss_generator.current_total;
        let zombie_refresh_count = zombie_generator.current_refresh_count;
        let boss_refresh_count = boss_generator.current_refresh_count;
        let zombie_timer = self.zombie_generator.get_node_as::<Timer>("Timer");
        let boss_timer = self.boss_generator.get_node_as::<Timer>("Timer");
        if zombie_timer.is_stopped()
            && boss_timer.is_stopped()
            && SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("1970-01-01 00:00:00 UTC was {} seconds ago!")
                .as_secs_f64()
                - RustPlayer::get_last_score_update()
                >= boss_timer.get_wait_time()
        {
            RustPlayer::reset_last_score_update();
            //30s内玩家未造成任何伤害，认为卡关了，实际上玩家击杀数足够，但击杀统计少了，强制刷新一批僵尸
            for _ in 0..zombie_total
                .saturating_sub(zombie_killed)
                .min(boss_refresh_count)
            {
                zombie_generator.generate_zombie();
            }
            for _ in 0..boss_total
                .saturating_sub(boss_killed)
                .min(zombie_refresh_count)
            {
                boss_generator.generate_zombie();
            }
            godot_print!(
                "Level{} is blocked, forcing a batch of zombies to be refreshed",
                self.level
            );
        }
        KILL_COUNT.store(zombie_killed, Ordering::Release);
        KILL_BOSS_COUNT.store(boss_killed, Ordering::Release);
        LIVE_COUNT.store(
            zombie_current
                .saturating_add(boss_current)
                .saturating_sub(killed),
            Ordering::Release,
        );
        drop(zombie_generator);
        drop(boss_generator);
        self.left_rampage_time = (self.left_rampage_time - delta as real).max(0.0);
        self.update_rampage_hud();
        self.update_progress_hud();
        self.update_fps_hud();
        if 0.0 == self.left_rampage_time && zombie_killed < zombie_current {
            RAMPAGE.store(true, Ordering::Release);
            self.play_rampage_bgm();
        } else if boss_killed < boss_current {
            self.play_boss_bgm();
        } else {
            RAMPAGE.store(false, Ordering::Release);
            self.play_bgm();
        }
        if zombie_killed >= zombie_total && boss_killed >= boss_total {
            self.level_up(false);
        }
    }

    fn ready(&mut self) {
        let gd = self.to_gd();
        self.bgm
            .signals()
            .finished()
            .connect_obj(&gd, Self::play_bgm);
        self.rampage_bgm
            .signals()
            .finished()
            .connect_obj(&gd, Self::play_rampage_bgm);
        self.boss_bgm
            .signals()
            .finished()
            .connect_obj(&gd, Self::play_boss_bgm);
        self.level_up(false);
    }

    fn input(&mut self, event: Gd<InputEvent>) {
        if event.is_action_pressed("l") {
            self.left_rampage_time = 0.0;
        } else if event.is_action_pressed("j") {
            //跳关
            self.level_up(true);
        }
    }
}

#[godot_api]
impl RustLevel {
    #[func]
    pub fn kill_confirmed(&mut self) {
        self.zombie_killed.fetch_add(1, Ordering::Release);
        self.update_progress_hud();
        RustPlayer::add_kill_count();
    }

    #[func]
    pub fn kill_boss_confirmed(&mut self) {
        self.boss_killed.fetch_add(1, Ordering::Release);
        self.update_progress_hud();
        RustPlayer::add_kill_boss_count();
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
        let boss_refreshed = self.boss_generator.bind().current;
        let zombie_refreshed = self.zombie_generator.bind().current;
        let boss_total = self.boss_generator.bind().current_total;
        let zombie_total = self.zombie_generator.bind().current_total;
        let mut label = self.get_center_container().get_node_as::<Label>("Progress");
        label.set_text(&format!(
            "PROGRESS {}+{}/{}+{}/{}+{}",
            self.boss_killed.load(Ordering::Acquire),
            self.zombie_killed.load(Ordering::Acquire),
            boss_refreshed,
            zombie_refreshed,
            boss_total,
            zombie_total
        ));
        label.show();
    }

    #[func]
    pub fn update_refresh_hud(&mut self) {
        let zombie_refresh_count = self.zombie_generator.bind().current_refresh_count;
        let zombie_timer = self.zombie_generator.get_node_as::<Timer>("Timer");
        let zombie_wait_time = zombie_timer.get_wait_time();
        let boss_refresh_count = self.boss_generator.bind().current_refresh_count;
        let boss_timer = self.boss_generator.get_node_as::<Timer>("Timer");
        let boss_wait_time = boss_timer.get_wait_time();
        let mut label = self.get_right_container().get_node_as::<Label>("Refresh");
        label.set_text(&format!(
            "REFRESH {} ZOMBIES IN {:.0}s {}\nREFRESH {} BOSS IN {:.0}s {}",
            zombie_refresh_count,
            zombie_wait_time,
            !zombie_timer.is_stopped(),
            boss_refresh_count,
            boss_wait_time,
            !boss_timer.is_stopped()
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

    pub fn level_up(&mut self, jump: bool) {
        RustPlayer::reset_last_score_update();
        let rate = self.grow_rate.powf(self.level as f32);
        self.level += 1;
        self.zombie_killed.store(0, Ordering::Release);
        self.boss_killed.store(0, Ordering::Release);
        self.left_rampage_time = self.rampage_time / rate;
        self.zombie_generator.bind_mut().level_up(
            jump,
            false,
            rate,
            ZOMBIE_REFRESH_BARRIER,
            ZOMBIE_MAX_SCREEN_COUNT,
        );
        self.boss_generator.bind_mut().level_up(
            jump,
            true,
            rate,
            BOSS_REFRESH_BARRIER,
            BOSS_MAX_SCREEN_COUNT,
        );
        self.update_level_hud();
        self.update_refresh_hud();
        self.update_progress_hud();
    }

    pub fn play_bgm(&mut self) {
        if self.bgm.is_playing() {
            return;
        }
        self.rampage_bgm.stop();
        self.boss_bgm.stop();
        self.bgm.play();
    }

    pub fn play_rampage_bgm(&mut self) {
        if self.rampage_bgm.is_playing() {
            return;
        }
        self.bgm.stop();
        self.boss_bgm.stop();
        self.rampage_bgm.play();
    }

    pub fn play_boss_bgm(&mut self) {
        if self.boss_bgm.is_playing() {
            return;
        }
        self.bgm.stop();
        self.rampage_bgm.stop();
        self.boss_bgm.play();
    }

    pub fn start(&mut self) {
        self.zombie_generator.bind_mut().start_timer();
        self.boss_generator.bind_mut().start_timer();
    }

    pub fn is_rampage() -> bool {
        RAMPAGE.load(Ordering::Acquire)
    }

    pub fn get_kill_count() -> u32 {
        KILL_COUNT.load(Ordering::Acquire)
    }

    pub fn get_kill_boss_count() -> u32 {
        KILL_BOSS_COUNT.load(Ordering::Acquire)
    }

    pub fn get_live_count() -> u32 {
        LIVE_COUNT.load(Ordering::Acquire)
    }
}

#[derive(GodotClass, Debug)]
#[class(base=Node)]
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
    refresh_barrier: u32,
    #[export]
    refresh_time: f64,
    #[export]
    max_screen_count: u32,
    #[export]
    zombie_scenes: Array<Gd<PackedScene>>,
    current_total: u32,
    current_refresh_count: u32,
    current: u32,
    current_refresh_barrier: u32,
    timer: OnReady<Gd<Timer>>,
    base: Base<Node>,
}

#[godot_api]
impl INode for ZombieGenerator {
    fn init(base: Base<Node>) -> Self {
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
            self.base()
                .get_parent()
                .unwrap()
                .cast::<Node>()
                .call_deferred("update_refresh_hud", &[]);
        } else if event.is_action_pressed("l") {
            RustPlayer::reset_last_score_update();
            while self.current < self.current_total
                && self.current.saturating_sub(RustLevel::get_kill_count()) < self.max_screen_count
            {
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
        boss: bool,
        rate: f32,
        refresh_barrier: u32,
        max_screen_count: u32,
    ) {
        self.current = 0;
        self.boss = boss;
        self.current_refresh_barrier = refresh_barrier;
        self.max_screen_count = max_screen_count;
        self.current_total = (self.total as f32 * rate) as u32;
        self.current_refresh_count = (self.refresh_count as f32 * rate) as u32;
        self.timer.start();
        if !jump && self.immediate {
            self.generate();
        }
    }

    pub fn start_timer(&mut self) {
        self.timer.start();
        self.base()
            .get_parent()
            .unwrap()
            .cast::<Node>()
            .call_deferred("update_refresh_hud", &[]);
    }

    #[func]
    pub fn generate(&mut self) {
        for _ in 0..self.current_refresh_count {
            let kill_count = if self.boss {
                RustLevel::get_kill_boss_count()
            } else {
                RustLevel::get_kill_count()
            };
            if 0 < kill_count
                && kill_count < self.current_refresh_barrier
                && self.current_refresh_count > self.refresh_barrier
                || self.current.saturating_sub(kill_count) >= self.max_screen_count
            {
                break;
            }
            if self.current >= self.current_total {
                self.timer.stop();
                break;
            }
            self.generate_zombie();
            self.current += 1;
        }
        while RustLevel::get_kill_count() >= self.current_refresh_barrier {
            self.current_refresh_barrier += self.refresh_barrier;
        }
    }

    pub fn generate_zombie(&self) {
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
            }
        }
    }
}
