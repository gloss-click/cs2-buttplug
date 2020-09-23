fn handle_update(update) {
    if update.player != () {
        for weapon in values(update.player.weapons) {
            let state = weapon.state.to_string();
            if state in ["Active", "Reloading"] {
                let current = weapon.ammo_clip;
                let max = weapon.ammo_clip_max;
                if current != () {
                    let frac_remaining = current.to_float() / max.to_float();
                    vibrate(1.0 - frac_remaining);
                }
            }
        }
    }
}
