let kills = 0;
let assists = 0;

fn handle_update(update, is_player) {
    if is_player && update.player != () {
        if update.player.match_stats != () {
            if kills < update.player.match_stats.kills {
                let kdiff = (update.player.match_stats.kills - kills);
                if kdiff > 5 {
                    kdiff = 5;
                }
                for i in range(0, kdiff) {
                    vibrate_index(kills.to_float()/50.0, 1.0, (kills + i) % 2);
                    linear(500, 1.0);
                    sleep(0.5);
                    linear(500, 0.0);
                }
            }

            kills = update.player.match_stats.kills;

            if assists < update.player.match_stats.assists {
                vibrate(0.15, 1.0);
                linear(500, 0.5);
            }
            
            assists = update.player.match_stats.assists;

            if update.player.match_stats.kills == 0 {
                kills = 0;
            }
            if update.player.match_stats.assists == 0 {
                assists = 0;
            }
        }
    }
}