use super::*;
use crate::{BOOMER_DAMAGE, BOOMER_REPEL, EXPLODE_AUDIOS};
use godot::builtin::Callable;
use godot::global::godot_error;

#[godot_api(secondary)]
impl RustBoomer {
    #[func]
    pub fn guard(&mut self) {
        if ZombieState::Dead == self.state {
            return;
        }
        self.animated_sprite2d.play_ex().name("guard").done();
        self.current_speed = self.speed * 0.2;
        self.state = ZombieState::Guard;
        if !self.guard_audio.is_playing() && self.guard_audio.is_inside_tree() {
            self.guard_audio.play();
        }
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
    }

    pub fn rampage(&mut self) {
        if ZombieState::Dead == self.state {
            return;
        }
        self.animated_sprite2d.play_ex().name("run").done();
        self.current_speed = self.speed * 1.6;
        self.state = ZombieState::Rampage;
        if !self.rampage_audio.is_playing() && self.rampage_audio.is_inside_tree() {
            self.rampage_audio.play();
        }
    }

    #[func]
    pub fn dying(&mut self) {
        self.attack_scream_audio.play();
        self.guard();
        self.state = ZombieState::Attack;
        self.get_alarm_progress().set_visible(false);
    }

    #[func]
    pub fn die(&mut self) {
        if ZombieState::Dead == self.state {
            return;
        }
        self.current_speed = 0.0;
        self.state = ZombieState::Dead;
        if self.die_audio.is_inside_tree() && self.detonable {
            //播放爆炸音效
            #[allow(clippy::borrow_interior_mutable_const)]
            if let Some(audio) = EXPLODE_AUDIOS.pick_random() {
                self.die_audio.set_stream(&audio);
                self.die_audio.play();
                self.die_flash.set_visible(true);
                self.die_flash.set_global_rotation_degrees(0.0);
                self.die_flash.play_ex().name("default").done();
                self.animated_sprite2d.queue_free();
            }
            let position = self.base().get_global_position();
            for mut body in self
                .zombie_damage_area
                .get_overlapping_bodies()
                .iter_shared()
            {
                if !body.is_instance_valid() {
                    continue;
                }
                if body.is_class("RustPlayer") {
                    body.cast::<RustPlayer>()
                        .bind_mut()
                        .on_hit(BOOMER_DAMAGE, position);
                } else if body.is_class("RustZombie")
                    || body.is_class("RustBoss")
                    || body.is_class("RustBoomer")
                {
                    if position != body.get_global_position() {
                        let direction = position.direction_to(body.get_global_position());
                        body.call_deferred(
                            "on_hit",
                            &[
                                BOOMER_DAMAGE.to_variant(),
                                direction.to_variant(),
                                BOOMER_REPEL.to_variant(),
                                position.to_variant(),
                            ],
                        );
                        RustPlayer::get().call_deferred("add_score", &[BOOMER_DAMAGE.to_variant()]);
                    }
                } else if body.is_class("RustGrenade") {
                    // ok
                } else {
                    godot_error!("Boomer hit an unexpected body: {}", body.get_class());
                }
            }
        }
        NOISE_POSITION.store(self.base().get_global_position());
        if let Some(mut tree) = self.base().get_tree() {
            if let Some(mut timer) = tree.create_timer(3.0) {
                timer.connect(
                    "timeout",
                    &Callable::from_sync_fn("clean_boomer_noise", |_| {
                        NOISE_POSITION.store(NO_NOISE);
                        Ok(().to_variant())
                    }),
                );
            }
        }
        // 释放资源
        self.hud.queue_free();
        self.head_shape2d.queue_free();
        self.collision_shape2d.queue_free();
        self.zombie_explode_area.queue_free();
        self.zombie_damage_area.queue_free();
        self.hit_audio.queue_free();
        self.blood_flash.queue_free();
        self.scream_audio.queue_free();
        self.guard_audio.queue_free();
        self.run_audio.queue_free();
        self.rampage_audio.queue_free();
        self.attack_scream_audio.queue_free();
        // 击杀僵尸确认
        if let Some(mut level) = RustLevel::get() {
            level.bind_mut().kill_confirmed();
        }
    }
}
