use super::*;
use crate::WEAPON_TEXTURE;
use crate::weapon::RustWeapon;
use godot::global::godot_warn;

#[godot_api(secondary)]
impl RustPlayer {
    pub fn change_weapon(&mut self, weapon_index: i32) {
        if PlayerState::Dead == self.state {
            return;
        }
        let weapon_count = self.weapons.get_child_count();
        if weapon_index >= weapon_count {
            if let Some(mut locked_label) = self.create_message() {
                locked_label
                    .bind_mut()
                    .show_message(&format!("WEAPON {} LOCKED", weapon_index + 1));
            }
            return;
        }
        for i in 0..weapon_count {
            if let Some(node) = self.weapons.get_child(i) {
                let mut weapon = node.cast::<RustWeapon>();
                if weapon_index == i {
                    weapon.set_visible(true);
                    let mut hud = self.hud.bind_mut();
                    let weapon_name = weapon.get_name().to_upper();
                    hud.update_weapon_name_hud(&weapon_name.to_string());
                    #[allow(clippy::borrow_interior_mutable_const)]
                    if let Some(weapon_texture) = WEAPON_TEXTURE.get(&weapon_name) {
                        hud.update_weapon_sprite_hud(weapon_texture);
                    } else {
                        hud.update_weapon_sprite_hud(Gd::null_arg());
                        godot_warn!("Weapon texture not found for: {}", weapon_name);
                    }
                    hud.update_ammo_hud(weapon.bind().get_ammo(), weapon.bind().get_clip());
                    weapon.bind_mut().weapon_ready();
                } else {
                    weapon.set_visible(false);
                    // 打断其他武器的换弹
                    weapon.bind_mut().stop_reload();
                }
            }
        }
        self.state = PlayerState::Guard;
        self.guard();
        if weapon_index == self.current_weapon_index {
            self.change_fail_audio.play();
            return;
        }
        self.animated_sprite2d.play_ex().name("guard").done();
        self.current_speed = self.speed * 0.75;
        self.current_weapon_index = weapon_index;
        if self.get_current_weapon().get_name() == "AWP".into() {
            self.zoom_audio.play();
            self.camera.set_zoom(Vector2::new(0.5, 0.5));
        } else {
            self.camera.set_zoom(Vector2::new(1.0, 1.0));
        }
        self.change_success_audio.play();
        // 更新HUD
        let mut rust_weapon = self.get_current_weapon();
        rust_weapon.bind_mut().update_ammo_hud();
        let mut hud = self.hud.bind_mut();
        hud.update_damage_hud(self.damage.saturating_add(rust_weapon.bind().get_damage()));
        hud.update_distance_hud(self.distance + rust_weapon.bind().get_distance());
        hud.update_repel_hud(self.repel + rust_weapon.bind().get_repel());
        hud.update_penetrate_hud(self.penetrate + rust_weapon.bind().get_penetrate());
    }

    pub fn get_current_weapon(&self) -> Gd<RustWeapon> {
        self.weapons
            .get_child(self.current_weapon_index)
            .expect("Weapon not configured")
            .cast::<RustWeapon>()
    }

    pub fn unlock_deagle(&mut self) {
        self.unlock_weapon("deagle", 1);
    }

    pub fn unlock_xm1014(&mut self) {
        self.unlock_weapon("xm1014", 2);
    }

    pub fn unlock_awp(&mut self) {
        self.unlock_weapon("awp", 3);
    }

    pub fn unlock_m79(&mut self) {
        self.unlock_weapon("m79", 4);
    }

    pub fn unlock_m4a1(&mut self) {
        self.unlock_weapon("m4a1", 5);
    }

    pub fn unlock_ak47(&mut self) {
        self.unlock_weapon("ak47", 6);
    }

    pub fn unlock_ak47_60r(&mut self) {
        self.unlock_weapon("ak47-60r", 7);
    }

    pub fn unlock_m249(&mut self) {
        self.unlock_weapon("m249", 8);
    }

    pub fn unlock_mg3(&mut self) {
        self.unlock_weapon("mg3", 9);
    }

    pub fn unlock_m134(&mut self) {
        self.unlock_weapon("m134", 10);
    }

    pub fn unlock_weapon(&mut self, weapon_name: &str, index: i32) {
        if self.weapons.get_child_count() > index {
            return;
        }
        if let Some(weapon) =
            load::<PackedScene>(&format!("res://scenes/weapons/{}.tscn", weapon_name))
                .try_instantiate_as::<RustWeapon>()
        {
            self.weapons.add_child(&weapon);
            if let Some(mut unlock_label) = self.create_message() {
                unlock_label.bind_mut().show_message(&format!(
                    "WEAPON {} UNLOCKED, PRESS {} TO USE IT",
                    weapon_name,
                    index + 1
                ));
            }
            self.change_weapon(index);
        }
    }
}
