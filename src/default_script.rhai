let current_weapon = "none";
let x = -1;
let falloff = 0.0;

let kills = 0;
let assists = 0;

fn handle_update(update) {
    if update.player != () {
        if update.player.match_stats != () {
            if kills < update.player.match_stats.kills {
                kills = update.player.match_stats.kills;
                vibrate(0.3 + kills.to_float()/25.0, 1.0);
            }
            if assists < update.player.match_stats.assists {
                assists = update.player.match_stats.assists;
                vibrate(0.15, 1.0);
            }

            if update.player.match_stats.kills == 0 {
                kills = 0;
            }
            if update.player.match_stats.assists == 0 {
                assists = 0;
            }
        }
    }
}
