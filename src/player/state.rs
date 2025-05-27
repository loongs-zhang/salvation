use super::*;
use crate::{PlayerState, random_bool};

static STATE: AtomicCell<PlayerState> = AtomicCell::new(PlayerState::Born);

#[godot_api(secondary)]
impl RustPlayer {
    #[func]
    pub fn born(&mut self) {
        if PlayerState::Dead != self.state || 0 == self.current_lives {
            return;
        }
        self.current_lives -= 1;
        self.weapons.set_visible(true);
        self.animated_sprite2d.play_ex().name("guard").done();
        self.current_speed = self.speed * self.get_current_weapon().bind().get_weight();
        self.state = PlayerState::Born;
        self.current_health = self.health;
        STATE.store(self.state);
        self.hud
            .bind_mut()
            .update_lives_hud(self.current_lives, self.lives);
        self.hud
            .bind_mut()
            .update_hp_hud(self.current_health, self.health);
    }

    #[func]
    pub fn guard(&mut self) {
        if PlayerState::Dead == self.state
            || PlayerState::Impact == self.state
            || PlayerState::Reload == self.state
            || PlayerState::Reloading == self.state
            || PlayerState::Chop == self.state
        {
            return;
        }
        self.weapons.set_visible(true);
        self.animated_sprite2d.play_ex().name("guard").done();
        self.current_speed = self.speed * self.get_current_weapon().bind().get_weight();
        self.state = PlayerState::Guard;
        STATE.store(self.state);
    }

    pub fn run(&mut self) {
        if PlayerState::Dead == self.state
            || PlayerState::Impact == self.state
            || PlayerState::Chop == self.state
        {
            return;
        }
        self.weapons.set_visible(false);
        self.animated_sprite2d.play_ex().name("run").done();
        self.current_speed = self.speed * 1.5 * self.get_current_weapon().bind().get_weight();
        self.state = PlayerState::Run;
        STATE.store(self.state);
        //打断换弹
        self.get_current_weapon().bind_mut().stop_reload();
        if !self.run_audio.is_playing() {
            self.run_audio.play();
        }
    }

    pub fn shoot(&mut self) {
        if PlayerState::Dead == self.state
            || PlayerState::Impact == self.state
            || PlayerState::Chop == self.state
        {
            return;
        }
        let mut rust_weapon = self.get_current_weapon();
        if rust_weapon.bind().must_reload() {
            // 没子弹时自动装填
            self.reload();
            return;
        }
        if !rust_weapon.bind().get_silenced() {
            //武器未消音
            NOISE_POSITION.store(rust_weapon.bind().get_noise_source());
        } else {
            NOISE_POSITION.store(NO_NOISE);
        }
        rust_weapon.set_visible(true);
        self.animated_sprite2d.play_ex().name("guard").done();
        self.current_speed = self.speed * 0.5 * rust_weapon.bind().get_weight();
        self.state = PlayerState::Shoot;
        STATE.store(self.state);
        //打断正在持续的换弹
        rust_weapon.bind_mut().stop_reload();
        rust_weapon
            .bind_mut()
            .fire(self.damage, self.distance, self.penetrate, self.repel);
    }

    pub fn headshot(&mut self) {
        self.headshot_audio.play();
    }

    pub fn chop(&mut self) {
        if PlayerState::Dead == self.state
            || PlayerState::Impact == self.state
            || self.current_chop_cooldown > 0.0
        {
            return;
        }
        self.current_chop_cooldown = self.chop_cooldown as f64;
        self.weapons.set_visible(false);
        self.animated_sprite2d.play_ex().name("chop").done();
        self.current_speed = self.speed * 0.75;
        self.state = PlayerState::Chop;
        STATE.store(self.state);
        //打断换弹
        self.get_current_weapon().bind_mut().stop_reload();
        let damage = 80 + self.damage;
        let repel = 30.0 + self.repel;
        self.knife.bind_mut().chop(damage, repel);
    }

    #[func]
    pub fn chopped(&mut self) {
        if PlayerState::Chop != self.state {
            return;
        }
        self.state = PlayerState::Guard;
        self.guard();
    }

    pub fn reload(&mut self) {
        let mut rust_weapon = self.get_current_weapon();
        if PlayerState::Dead == self.state
            || PlayerState::Impact == self.state
            || PlayerState::Reload == self.state
            || PlayerState::Chop == self.state
            || !rust_weapon.bind_mut().reload()
        {
            return;
        }
        self.weapons.set_visible(true);
        self.animated_sprite2d.play_ex().name("reload").done();
        self.current_speed = self.speed * 0.75 * rust_weapon.bind().get_weight();
        self.state = PlayerState::Reload;
        STATE.store(self.state);
    }

    #[func]
    pub fn reloading(&mut self) {
        if PlayerState::Dead == self.state || PlayerState::Impact == self.state {
            return;
        }
        self.animated_sprite2d.play_ex().name("reload").done();
        self.current_speed = self.speed * 0.75 * self.get_current_weapon().bind().get_weight();
        self.state = PlayerState::Reloading;
        STATE.store(self.state);
    }

    #[func]
    pub fn reloaded(&mut self) {
        if PlayerState::Dead == self.state || PlayerState::Impact == self.state {
            return;
        }
        self.state = PlayerState::Guard;
        self.guard();
    }

    pub fn hit(&mut self, hit_position: Vector2) {
        if PlayerState::Dead == self.state {
            return;
        }
        self.weapons.set_visible(false);
        self.animated_sprite2d.play_ex().name("hit").done();
        self.current_speed = self.speed * 0.5 * self.get_current_weapon().bind().get_weight();
        self.state = PlayerState::Hit;
        let player_position = self.base().get_global_position();
        self.blood_flash.set_global_position(
            player_position + player_position.direction_to(hit_position).normalized() * 18.0,
        );
        self.blood_flash.look_at(hit_position);
        self.blood_flash.restart();
        STATE.store(self.state);
        if random_bool() {
            self.body_hurt.play();
        } else {
            self.bone_hurt.play();
        }
        if !self.scream_audio.is_playing() {
            self.scream_audio.play();
        }
    }

    pub fn on_impact(&mut self, hit_val: i64, impact_position: Vector2) {
        if PlayerState::Dead == self.state {
            return;
        }
        self.on_hit(hit_val, impact_position);
        if PlayerState::Dead == self.state {
            return;
        }
        self.weapons.set_visible(false);
        self.animated_sprite2d.play_ex().name("bump").done();
        self.current_speed = self.speed * 1.25;
        self.state = PlayerState::Impact;
        let player_position = self.base().get_global_position();
        self.blood_flash.set_global_position(
            player_position + player_position.direction_to(impact_position).normalized() * 18.0,
        );
        self.blood_flash.look_at(impact_position);
        self.blood_flash.set_one_shot(false);
        self.blood_flash.set_emitting(true);
        self.blood_flash.restart();
        STATE.store(self.state);
        //打断正在持续的换弹
        self.get_current_weapon().bind_mut().stop_reload();
        IMPACT_POSITION.store(impact_position);
    }

    pub fn impacting(&mut self) {
        if PlayerState::Impact != self.state {
            return;
        }
        self.animated_sprite2d.play_ex().name("bump").done();
        self.current_speed = self.speed * 1.25;
        self.state = PlayerState::Impact;
        STATE.store(self.state);
        let hit_position = IMPACT_POSITION.load();
        self.base_mut().look_at(hit_position);
        if !self.scream_audio.is_playing() {
            self.scream_audio.play();
        }
    }

    pub fn impacted(&mut self) {
        if PlayerState::Impact != self.state {
            return;
        }
        self.state = PlayerState::Guard;
        self.blood_flash.set_emitting(false);
        self.blood_flash.set_one_shot(true);
        self.blood_flash.restart();
        IMPACT_POSITION.store(Vector2::ZERO);
        IMPACTING.store(0.0);
        self.guard();
    }

    pub fn die(&mut self, hit_position: Vector2) {
        if PlayerState::Dead == self.state {
            return;
        }
        self.weapons.set_visible(false);
        self.animated_sprite2d.look_at(hit_position);
        self.animated_sprite2d.play_ex().name("die").done();
        self.current_speed = 0.0;
        self.state = PlayerState::Dead;
        STATE.store(self.state);
        //打断换弹
        self.get_current_weapon().bind_mut().stop_reload();
        self.die_audio.play();
        DIED.fetch_add(1, Ordering::Release);
        if 0 == self.current_lives {
            if let Some(tree) = self.base().get_tree() {
                if let Some(root) = tree.get_root() {
                    root.get_node_as::<RustWorld>("RustWorld")
                        .signals()
                        .player_dead()
                        .emit();
                }
            }
            return;
        }
        if let Some(mut tree) = self.base().get_tree() {
            if let Some(mut timer) = tree.create_timer(3.0) {
                timer.connect("timeout", &self.base().callable("born"));
            }
        }
    }

    pub fn get_state() -> PlayerState {
        STATE.load()
    }
}
