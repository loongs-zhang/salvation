use crate::hud::RustHUD;
use crate::level::generator::ZombieGenerator;
use crate::player::RustPlayer;
use crate::save::RustSaveLoader;
use crate::world::RustWorld;
use crate::{
    BOOMER_MAX_SCREEN_COUNT, BOOMER_REFRESH_BARRIER, BOSS_MAX_SCREEN_COUNT, BOSS_REFRESH_BARRIER,
    LEVEL_GROW_RATE, LEVEL_RAMPAGE_TIME, PITCHER_MAX_SCREEN_COUNT, PITCHER_REFRESH_BARRIER,
    ZOMBIE_MAX_SCREEN_COUNT, ZOMBIE_REFRESH_BARRIER, kill_all_zombies,
};
use godot::builtin::real;
use godot::classes::{AudioStreamPlayer2D, INode2D, InputEvent, Node2D, Timer};
use godot::global::godot_warn;
use godot::meta::ToGodot;
use godot::obj::{Base, Gd, OnReady, WithBaseField};
use godot::register::{GodotClass, godot_api};
use rand::Rng;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

pub mod generator;

pub mod save;

//todo 僵尸对象池优化，僵尸被清理后不是free而是返回对象池
static RAMPAGE: AtomicBool = AtomicBool::new(false);

static LIVE_COUNT: AtomicU32 = AtomicU32::new(0);

#[derive(GodotClass)]
#[class(base=Node2D)]
pub struct RustLevel {
    #[doc = "是否开启地狱模式"]
    #[export]
    hell: bool,
    #[doc = "当前关卡难度等级"]
    #[export]
    level: u32,
    #[doc = "下一关强度提升系数"]
    #[export]
    grow_rate: real,
    #[doc = "僵尸狂暴倒计时"]
    #[export]
    rampage_time: real,
    #[doc = "普通僵尸刷新间隔"]
    #[export]
    zombie_refresh_time: f64,
    #[doc = "自爆者刷新间隔"]
    #[export]
    boomer_refresh_time: f64,
    #[doc = "投掷者刷新间隔"]
    #[export]
    pitcher_refresh_time: f64,
    #[doc = "BOSS刷新间隔"]
    #[export]
    boss_refresh_time: f64,
    left_rampage_time: real,
    zombie_generator: OnReady<Gd<ZombieGenerator>>,
    boomer_generator: OnReady<Gd<ZombieGenerator>>,
    pitcher_generator: OnReady<Gd<ZombieGenerator>>,
    boss_generator: OnReady<Gd<ZombieGenerator>>,
    bgm: OnReady<Gd<AudioStreamPlayer2D>>,
    rampage_bgm: OnReady<Gd<AudioStreamPlayer2D>>,
    boss_bgm: OnReady<Gd<AudioStreamPlayer2D>>,
    base: Base<Node2D>,
}

#[godot_api]
impl INode2D for RustLevel {
    fn init(base: Base<Node2D>) -> Self {
        Self {
            hell: false,
            level: 0,
            grow_rate: LEVEL_GROW_RATE,
            rampage_time: LEVEL_RAMPAGE_TIME,
            zombie_refresh_time: 3.0,
            boomer_refresh_time: 10.0,
            pitcher_refresh_time: 10.0,
            boss_refresh_time: 60.0,
            left_rampage_time: LEVEL_RAMPAGE_TIME,
            zombie_generator: OnReady::from_node("ZombieGenerator"),
            boomer_generator: OnReady::from_node("BoomerGenerator"),
            pitcher_generator: OnReady::from_node("PitcherGenerator"),
            boss_generator: OnReady::from_node("BossGenerator"),
            bgm: OnReady::from_node("Bgm"),
            rampage_bgm: OnReady::from_node("RampageBgm"),
            boss_bgm: OnReady::from_node("BossBgm"),
            base,
        }
    }

    fn process(&mut self, delta: f64) {
        let player_position = RustPlayer::get_position();
        self.bgm.set_global_position(player_position);
        self.rampage_bgm.set_global_position(player_position);
        self.boss_bgm.set_global_position(player_position);
        if RustWorld::is_paused() {
            return;
        }
        let zombie_killed = self.get_zombie_killed();
        let boss_killed = self.get_boss_killed();
        let killed = zombie_killed.saturating_add(boss_killed);
        let zombie_current = self.get_zombie_current();
        let boss_current = self.get_boss_current();
        let zombie_total = self.get_zombie_total();
        let boss_total = self.get_boss_total();
        let zombie_refresh_count = self.get_zombie_current_refresh_count();
        let boss_refresh_count = self.get_boss_current_refresh_count();
        if self.is_stopped()
            && SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("1970-01-01 00:00:00 UTC was {} seconds ago!")
                .as_secs()
                - RustPlayer::get_last_score_update()
                >= 20
        {
            RustPlayer::reset_last_score_update();
            //20s内玩家未造成任何伤害，认为卡关了，实际上玩家击杀数足够，但击杀统计少了，强制刷新一批僵尸
            let refresh_zombie_count = zombie_total
                .saturating_sub(zombie_killed)
                .min(zombie_refresh_count);
            let refresh_boss_count = boss_total
                .saturating_sub(boss_killed)
                .min(boss_refresh_count);
            for _ in 0..refresh_zombie_count {
                match rand::thread_rng().gen_range(-1..=2) {
                    // 生成投手僵尸
                    -1 => self.pitcher_generator.bind().generate_zombie(),
                    // 生成爆炸僵尸
                    0 => self.boomer_generator.bind().generate_zombie(),
                    // 生成普通僵尸
                    _ => self.zombie_generator.bind().generate_zombie(),
                }
            }
            for _ in 0..refresh_boss_count {
                self.boss_generator.bind().generate_zombie();
            }
            godot_warn!(
                "Level{} is blocked, forcing refresh {} zombies and {} bosses",
                self.level,
                refresh_zombie_count,
                refresh_boss_count
            );
        }
        LIVE_COUNT.store(
            zombie_current
                .saturating_add(boss_current)
                .saturating_sub(killed),
            Ordering::Release,
        );
        self.left_rampage_time = (self.left_rampage_time - delta as real).max(0.0);
        self.update_rampage_hud();
        self.update_progress_hud();
        if 0.0 == self.left_rampage_time && zombie_killed < zombie_current {
            RAMPAGE.store(true, Ordering::Release);
            if !self.hell {
                self.boomer_generator
                    .bind_mut()
                    .refresh_timer(self.boomer_refresh_time / 2.0);
                self.pitcher_generator
                    .bind_mut()
                    .refresh_timer(self.pitcher_refresh_time / 2.0);
                self.boss_generator
                    .bind_mut()
                    .refresh_timer(self.boss_refresh_time / 6.0);
            }
            self.play_rampage_bgm();
        } else if boss_killed < boss_current {
            self.play_boss_bgm();
        } else {
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
        } else if cfg!(feature = "develop") && event.is_action_pressed("j") {
            //在这一帧清理所有僵尸
            kill_all_zombies();
            //下一帧跳关
            self.base_mut()
                .call_deferred("level_up", &[true.to_variant()]);
        }
    }
}

#[godot_api]
impl RustLevel {
    pub fn update_level_hud(&mut self) {
        RustHUD::get().bind_mut().update_level_hud(self.level);
    }

    pub fn update_rampage_hud(&mut self) {
        RustHUD::get()
            .bind_mut()
            .update_rampage_hud(self.left_rampage_time);
    }

    #[func]
    pub fn update_progress_hud(&mut self) {
        RustHUD::get().bind_mut().update_progress_hud(
            self.get_boss_killed(),
            self.get_zombie_killed(),
            self.get_boss_current(),
            self.get_zombie_current(),
            self.get_boss_total(),
            self.get_zombie_total(),
        );
    }

    #[func]
    pub fn update_refresh_hud(&mut self) {
        let mut hud = RustHUD::get();
        for child in self.base().get_children().iter_shared() {
            if child.is_class("ZombieGenerator") {
                let generator_type = child.get_name().replace("Generator", "").to_lower();
                let zombie_generator = child.cast::<ZombieGenerator>();
                let zombie_refresh_count = zombie_generator
                    .bind()
                    .current_refresh_count
                    .min(zombie_generator.bind().refresh_barrier);
                let zombie_timer = zombie_generator.get_node_as::<Timer>("Timer");
                let zombie_wait_time = zombie_timer.get_wait_time();
                hud.call_deferred(
                    &format!("update_refresh_{}_hud", generator_type),
                    &[
                        zombie_timer.is_stopped().to_variant(),
                        zombie_refresh_count.to_variant(),
                        zombie_wait_time.to_variant(),
                    ],
                );
            }
        }
    }

    pub fn is_stopped(&self) -> bool {
        for child in self.base().get_children().iter_shared() {
            if child.is_class("ZombieGenerator") {
                let zombie_generator = child.cast::<ZombieGenerator>();
                if zombie_generator.get_node_as::<Timer>("Timer").is_stopped() {
                    return true;
                }
            }
        }
        false
    }

    pub fn get_zombie_total(&self) -> u32 {
        let mut zombie_total = 0;
        for child in self.base().get_children().iter_shared() {
            if child.is_class("ZombieGenerator") && !child.get_name().contains("Boss") {
                let zombie_generator = child.cast::<ZombieGenerator>();
                zombie_total += zombie_generator.bind().current_total;
            }
        }
        zombie_total
    }

    pub fn get_zombie_current(&self) -> u32 {
        let mut zombie_current = 0;
        for child in self.base().get_children().iter_shared() {
            if child.is_class("ZombieGenerator") && !child.get_name().contains("Boss") {
                let zombie_generator = child.cast::<ZombieGenerator>();
                zombie_current += zombie_generator.bind().current;
            }
        }
        zombie_current
    }

    pub fn get_zombie_killed(&self) -> u32 {
        let mut zombie_killed = 0;
        for child in self.base().get_children().iter_shared() {
            if child.is_class("ZombieGenerator") && !child.get_name().contains("Boss") {
                let zombie_generator = child.cast::<ZombieGenerator>();
                zombie_killed += zombie_generator.bind().get_killed();
            }
        }
        zombie_killed
    }

    pub fn get_zombie_current_refresh_count(&self) -> u32 {
        let mut current_refresh_count = 0;
        for child in self.base().get_children().iter_shared() {
            if child.is_class("ZombieGenerator") && !child.get_name().contains("Boss") {
                let zombie_generator = child.cast::<ZombieGenerator>();
                current_refresh_count += zombie_generator.bind().current_refresh_count;
            }
        }
        current_refresh_count
    }

    pub fn get_boss_total(&self) -> u32 {
        let mut boss_total = 0;
        for child in self.base().get_children().iter_shared() {
            if child.is_class("ZombieGenerator") && child.get_name().contains("Boss") {
                let zombie_generator = child.cast::<ZombieGenerator>();
                boss_total += zombie_generator.bind().current_total;
            }
        }
        boss_total
    }

    pub fn get_boss_current(&self) -> u32 {
        let mut zombie_current = 0;
        for child in self.base().get_children().iter_shared() {
            if child.is_class("ZombieGenerator") && child.get_name().contains("Boss") {
                let zombie_generator = child.cast::<ZombieGenerator>();
                zombie_current += zombie_generator.bind().current;
            }
        }
        zombie_current
    }

    pub fn get_boss_current_refresh_count(&self) -> u32 {
        let mut current_refresh_count = 0;
        for child in self.base().get_children().iter_shared() {
            if child.is_class("ZombieGenerator") && child.get_name().contains("Boss") {
                let zombie_generator = child.cast::<ZombieGenerator>();
                current_refresh_count += zombie_generator.bind().current_refresh_count;
            }
        }
        current_refresh_count
    }

    pub fn get_boss_killed(&self) -> u32 {
        let mut zombie_current = 0;
        for child in self.base().get_children().iter_shared() {
            if child.is_class("ZombieGenerator") && child.get_name().contains("Boss") {
                let zombie_generator = child.cast::<ZombieGenerator>();
                zombie_current += zombie_generator.bind().get_killed();
            }
        }
        zombie_current
    }

    #[func]
    pub fn level_up(&mut self, jump: bool) {
        // clean extra zombies
        for mut node in self
            .base()
            .get_tree()
            .unwrap()
            .get_nodes_in_group("zombie")
            .iter_shared()
        {
            if !node.has_method("before_load") {
                continue;
            }
            node.call("before_load", &[]);
        }
        if self.level > 1 {
            RustSaveLoader::get().call_deferred("save_game", &[]);
        }
        RAMPAGE.store(false, Ordering::Release);
        RustPlayer::reset_last_score_update();
        let rate = self.grow_rate.powf(self.level as f32);
        self.level += 1;
        // 加强M4A1，不然后续的消音没什么意义
        // self.left_rampage_time = self.rampage_time / rate;
        self.left_rampage_time = self.rampage_time;
        self.zombie_generator.bind_mut().level_up(
            jump,
            rate,
            ZOMBIE_REFRESH_BARRIER,
            if self.hell {
                (ZOMBIE_MAX_SCREEN_COUNT as real * 1.6) as u32
            } else {
                ZOMBIE_MAX_SCREEN_COUNT
            },
            self.zombie_refresh_time,
        );
        self.boomer_generator.bind_mut().level_up(
            jump,
            rate,
            BOOMER_REFRESH_BARRIER,
            if self.hell {
                (BOOMER_MAX_SCREEN_COUNT as real * 1.6) as u32
            } else {
                BOOMER_MAX_SCREEN_COUNT
            },
            self.boomer_refresh_time,
        );
        self.pitcher_generator.bind_mut().level_up(
            jump,
            rate,
            PITCHER_REFRESH_BARRIER,
            if self.hell {
                (PITCHER_MAX_SCREEN_COUNT as real * 1.6) as u32
            } else {
                PITCHER_MAX_SCREEN_COUNT
            },
            self.pitcher_refresh_time,
        );
        self.boss_generator.bind_mut().level_up(
            jump,
            rate,
            BOSS_REFRESH_BARRIER,
            BOSS_MAX_SCREEN_COUNT,
            self.boss_refresh_time,
        );
        self.update_level_hud();
        self.update_refresh_hud();
        self.update_progress_hud();
        self.unlock_weapons();
    }

    fn unlock_weapons(&mut self) {
        // 这里的判断不加else，恢复存档时方便解锁武器
        let mut player = RustPlayer::get();
        if self.level >= 33 {
            // 相对强力的武器
            player.call_deferred("unlock_skull_5", &[]);
        }
        if self.level >= 32 {
            // 相对强力的武器
            player.call_deferred("unlock_xm1134", &[]);
        }
        if self.level >= 31 {
            player.call_deferred("unlock_m32", &[]);
        }
        if self.level >= 30 {
            player.call_deferred("unlock_m134", &[]);
        }
        if self.level >= 29 {
            player.call_deferred("unlock_m95", &[]);
        }
        if self.level >= 28 {
            // 相对强力的武器
            player.call_deferred("unlock_skull_6", &[]);
        }
        if self.level >= 27 {
            player.call_deferred("unlock_mg3", &[]);
        }
        if self.level >= 26 {
            player.call_deferred("unlock_m249", &[]);
        }
        if self.level >= 25 {
            player.call_deferred("unlock_rpg_7", &[]);
        }
        if self.level >= 24 {
            // 相对强力的武器
            player.call_deferred("unlock_ak47_60r", &[]);
        }
        if self.level >= 21 {
            player.call_deferred("unlock_xm1014", &[]);
        }
        if self.level >= 18 {
            player.call_deferred("unlock_ak47", &[]);
        }
        if self.level >= 15 {
            player.call_deferred("unlock_m4a1", &[]);
        }
        if self.level >= 12 {
            // 相对强力的武器
            player.call_deferred("unlock_m79", &[]);
        }
        if self.level >= 9 {
            player.call_deferred("unlock_awp", &[]);
        }
        if self.level >= 6 {
            player.call_deferred("unlock_m1887", &[]);
        }
        if self.level >= 3 {
            player.call_deferred("unlock_deagle", &[]);
        }
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

    pub fn reset(&mut self) {
        self.level = 0;
        self.level_up(false);
    }

    pub fn start(&mut self) {
        RustPlayer::reset_last_score_update();
        for mut child in self.base().get_children().iter_shared() {
            if child.is_class("ZombieGenerator") {
                child.call("start_timer", &[]);
            }
        }
    }

    pub fn stop(&mut self) {
        RustPlayer::reset_last_score_update();
        for mut child in self.base().get_children().iter_shared() {
            if child.is_class("ZombieGenerator") {
                child.call("stop_timer", &[]);
            }
        }
    }

    pub fn enable_hell(&mut self) {
        self.zombie_refresh_time = 0.2;
        self.boomer_refresh_time = 0.5;
        self.pitcher_refresh_time = 0.5;
        self.boss_refresh_time = 2.0;
        self.rampage_time = 0.0;
        self.left_rampage_time = self.rampage_time;
        self.hell = true;
        self.zombie_generator
            .bind_mut()
            .refresh_timer(self.zombie_refresh_time);
        self.boomer_generator
            .bind_mut()
            .refresh_timer(self.boomer_refresh_time);
        self.pitcher_generator
            .bind_mut()
            .refresh_timer(self.pitcher_refresh_time);
        self.boss_generator
            .bind_mut()
            .refresh_timer(self.boss_refresh_time);
    }

    pub fn is_rampage() -> bool {
        RAMPAGE.load(Ordering::Acquire)
    }

    pub fn get_live_count() -> u32 {
        LIVE_COUNT.load(Ordering::Acquire)
    }

    pub fn get() -> Option<Gd<Self>> {
        RustWorld::get().try_get_node_as::<Self>("RustLevel")
    }
}
