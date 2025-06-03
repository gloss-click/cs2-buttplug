let ammo_last = 0;

fn handle_update(update, is_player) {
    if is_player && update.player != () {
        for weapon in update.player.weapons.values() {
            if weapon.state.to_string() == "Active" {
                if weapon.ammo_clip != () {
                    if ammo_last != weapon.ammo_clip {
                        vibrate(1.0 - (weapon.ammo_clip.to_float() / weapon.ammo_clip_max.to_float()), 0.5);
                    }
                    ammo_last = weapon.ammo_clip;
                }
            }
        }
    }
}