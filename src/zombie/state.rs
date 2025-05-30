use super::*;
use crate::ZOMBIE_REFRESH_BARRIER;
use crate::world::ground::RustGround;

#[godot_api(secondary)]
impl RustZombie {
    fn notify_animation(&mut self) {
        self.animated_sprite2d
            .signals()
            .change_zombie_state()
            .emit(self.state);
    }

    #[func]
    pub fn guard(&mut self) {
        if ZombieState::Dead == self.state {
            return;
        }
        self.animated_sprite2d.play_ex().name("guard").done();
        self.current_speed = self.speed * 0.2;
        self.state = ZombieState::Guard;
        if !self.guard_audio.is_playing() && self.guard_audio.is_inside_tree() {
            if RustLevel::get_live_count() >= ZOMBIE_REFRESH_BARRIER {
                self.guard_audio.set_volume_db(-30.0);
            } else {
                self.guard_audio.set_volume_db(-20.0);
            }
            self.guard_audio.play();
        }
        self.notify_animation();
    }

    pub fn run(&mut self) {
        if ZombieState::Dead == self.state {
            return;
        }
        self.animated_sprite2d.play_ex().name("run").done();
        self.current_speed = self.speed * 1.35;
        self.state = ZombieState::Run;
        if !self.run_audio.is_playing() && self.run_audio.is_inside_tree() {
            self.run_audio.play();
        }
        self.notify_animation();
    }

    pub fn hit(&mut self, direction: Vector2, hit_position: Vector2) {
        if ZombieState::Dead == self.state {
            return;
        }
        self.animated_sprite2d.play_ex().name("guard").done();
        self.current_speed = self.speed * 0.1;
        self.state = ZombieState::Hit;
        if self.current_flash_cooldown <= 0.0 {
            self.blood_flash.set_global_position(hit_position);
            self.blood_flash.look_at(hit_position - direction);
            self.blood_flash.restart();
            self.current_flash_cooldown = self.blood_flash.get_lifetime() * 0.25;
        }
        if self.hit_audio.is_inside_tree() {
            self.hit_audio.play();
        }
        if self.scream_audio.is_inside_tree() {
            self.scream_audio.play();
        }
        self.notify_animation();
    }

    pub fn rampage(&mut self) {
        if ZombieState::Dead == self.state {
            return;
        }
        self.animated_sprite2d.play_ex().name("run").done();
        self.current_speed = self.speed * 1.6;
        self.state = ZombieState::Rampage;
        if !self.rampage_audio.is_playing() && self.rampage_audio.is_inside_tree() {
            let live_count = RustLevel::get_live_count();
            if live_count >= ZOMBIE_REFRESH_BARRIER {
                self.rampage_audio.set_volume_db(-40.0);
            } else if live_count >= ZOMBIE_REFRESH_BARRIER / 2 {
                self.rampage_audio.set_volume_db(-25.0);
            } else {
                self.rampage_audio.set_volume_db(-12.0);
            }
            self.rampage_audio.play();
        }
        self.notify_animation();
    }

    pub fn attack(&mut self) {
        if ZombieState::Dead == self.state || !self.attackable {
            return;
        }
        self.base_mut().look_at(RustPlayer::get_position());
        self.animated_sprite2d.play_ex().name("attack").done();
        self.current_speed = self.speed * 0.5;
        self.state = ZombieState::Attack;
        let direction = NEXT_ATTACK_DIRECTION.load()
            + self
                .base()
                .get_global_position()
                .direction_to(RustPlayer::get_position());
        NEXT_ATTACK_DIRECTION.store(direction.normalized());
        if self.attack_audio.is_inside_tree() {
            self.attack_audio.play();
        }
        if !self.attack_scream_audio.is_playing() && self.attack_scream_audio.is_inside_tree() {
            self.attack_scream_audio.play();
        }
        self.notify_animation();
    }

    #[func]
    pub fn die(&mut self) {
        if ZombieState::Dead == self.state {
            return;
        }
        self.animated_sprite2d.play_ex().name("die").done();
        self.current_speed = 0.0;
        self.state = ZombieState::Dead;
        if self.die_audio.is_inside_tree() {
            self.die_audio.play();
        }
        // 释放资源
        self.base_mut()
            .set_z_index(RustGround::get_objects_z_index());
        self.hud.queue_free();
        self.head_shape2d.queue_free();
        self.collision_shape2d.queue_free();
        self.zombie_attack_area.queue_free();
        self.zombie_damage_area.queue_free();
        self.hit_audio.queue_free();
        self.blood_flash.queue_free();
        self.scream_audio.queue_free();
        self.guard_audio.queue_free();
        self.run_audio.queue_free();
        self.rampage_audio.queue_free();
        self.attack_audio.queue_free();
        self.attack_scream_audio.queue_free();
        self.notify_animation();
        // 45S后自动清理尸体
        BODY_COUNT.fetch_add(1, Ordering::Release);
        if let Some(mut tree) = self.base().get_tree() {
            if let Some(mut timer) = tree.create_timer(45.0) {
                timer.connect("timeout", &self.base().callable("clean_body"));
            }
        }
        // 击杀僵尸确认
        if let Some(mut level) = RustLevel::get() {
            level.bind_mut().kill_confirmed();
        }
    }
}
