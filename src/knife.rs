use crate::boss::RustBoss;
use crate::common::RustMessage;
use crate::player::RustPlayer;
use crate::zombie::RustZombie;
use godot::builtin::{Array, real};
use godot::classes::tween::{EaseType, TransitionType};
use godot::classes::{
    Area2D, AudioStream, AudioStreamPlayer2D, IArea2D, Node, Node2D, PackedScene,
};
use godot::meta::ToGodot;
use godot::obj::{Base, Gd, OnReady, WithBaseField};
use godot::prelude::load;
use godot::register::{GodotClass, godot_api};

#[derive(GodotClass)]
#[class(base=Area2D)]
pub struct RustKnife {
    #[export]
    max_attack_angle: real,
    final_repel: real,
    final_damage: i64,
    damage_area: OnReady<Gd<Area2D>>,
    chop_audio: OnReady<Gd<AudioStreamPlayer2D>>,
    hit_audio: OnReady<Gd<AudioStreamPlayer2D>>,
    hit_audios: Array<Gd<AudioStream>>,
    message_scene: OnReady<Gd<PackedScene>>,
    base: Base<Area2D>,
}

#[godot_api]
impl IArea2D for RustKnife {
    fn init(base: Base<Area2D>) -> Self {
        Self {
            max_attack_angle: 60.0,
            final_repel: 0.0,
            final_damage: 0,
            damage_area: OnReady::from_node("DamageArea"),
            chop_audio: OnReady::from_node("ChopAudio"),
            hit_audio: OnReady::from_node("HitAudio"),
            hit_audios: Array::new(),
            message_scene: OnReady::from_loaded("res://scenes/rust_message.tscn"),
            base,
        }
    }

    fn ready(&mut self) {
        if self.hit_audios.is_empty() {
            for i in 1..=2 {
                self.hit_audios.push(&load(&format!(
                    "res://asserts/player/knifes/katana/katana_hit{}.wav",
                    i
                )));
            }
        }
        let gd = self.to_gd();
        self.damage_area
            .signals()
            .body_entered()
            .connect_obj(&gd, Self::on_area_2d_body_entered);
    }
}

#[godot_api]
impl RustKnife {
    pub fn chop(&mut self, final_damage: i64, final_repel: real) {
        if self.base().is_visible() {
            return;
        }
        self.final_damage = final_damage;
        self.final_repel = final_repel;
        let max_attack_angle = self.max_attack_angle;
        self.base_mut()
            .set_global_rotation_degrees(-max_attack_angle);
        self.base_mut().set_visible(true);
        let mut tween = self
            .base_mut()
            .create_tween()
            .expect("Failed to create tween");
        tween.set_ease(EaseType::IN_OUT);
        tween.set_trans(TransitionType::QUAD);
        tween
            .tween_property(
                &self.base.to_gd(),
                "rotation",
                &max_attack_angle.to_radians().to_variant(),
                0.2,
            )
            .expect("tween failed")
            .from(&(-max_attack_angle).to_radians().to_variant());
        tween.tween_callback(&self.base().callable("hide"));
        self.chop_audio.play();
    }

    #[func]
    pub fn on_area_2d_body_entered(&mut self, body: Gd<Node2D>) {
        if !self.base().is_visible() {
            return;
        }
        let position = self.base().get_global_position();
        let mut damage = self.final_damage;
        if body.is_class("RustZombie") {
            if let Some(audio) = self.hit_audios.pick_random() {
                self.hit_audio.set_stream(&audio);
                self.hit_audio.play();
            }
            let mut zombie = body.cast::<RustZombie>();
            let zombie_position = zombie.get_global_position();
            let direction = position.direction_to(zombie_position);
            // 暗杀判定
            if self.try_assassinate(&zombie) {
                damage *= 3;
                if let Some(mut assassinate_label) =
                    self.message_scene.try_instantiate_as::<RustMessage>()
                {
                    assassinate_label.set_global_position(position);
                    if let Some(tree) = self.base().get_tree() {
                        if let Some(mut root) = tree.get_root() {
                            root.add_child(&assassinate_label);
                            assassinate_label.bind_mut().show_message("ASSASSINATE");
                        }
                    }
                }
            }
            zombie.bind_mut().on_hit(
                damage,
                direction,
                self.final_repel,
                zombie_position + direction,
            );
            if damage > 0 {
                RustPlayer::add_score(damage as u64);
            }
        } else if body.is_class("RustBoss") {
            if let Some(audio) = self.hit_audios.pick_random() {
                self.hit_audio.set_stream(&audio);
                self.hit_audio.play();
            }
            let mut boss = body.cast::<RustBoss>();
            let boss_position = boss.get_global_position();
            let direction = position.direction_to(boss_position);
            boss.bind_mut().on_hit(
                damage,
                direction,
                self.final_repel,
                boss_position + direction,
            );
            if damage > 0 {
                RustPlayer::add_score(damage as u64);
            }
        }
    }

    #[func]
    pub fn hide(&mut self) {
        self.base_mut().set_visible(false);
        self.get_rust_player().bind_mut().chopped();
    }

    // 僵尸背对玩家，则判定可暗杀
    pub fn try_assassinate(&self, zombie: &Gd<RustZombie>) -> bool {
        let zombie_position = zombie.get_global_position();
        let player_position = RustPlayer::get_position();
        let to_player_dir = player_position.direction_to(zombie_position).normalized();
        let angle = zombie
            .bind()
            .get_current_direction()
            .angle_to(to_player_dir)
            .to_degrees();
        (-60.0..=60.0).contains(&angle)
    }

    pub fn get_rust_player(&mut self) -> Gd<RustPlayer> {
        if let Some(tree) = self.base().get_tree() {
            if let Some(root) = tree.get_root() {
                return root
                    .get_node_as::<Node>("RustWorld")
                    .get_node_as::<RustPlayer>("RustPlayer");
            }
        }
        panic!("RustPlayer not found");
    }
}
