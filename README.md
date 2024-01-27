# cs2-buttplug: The bomb has been planted

I've updated [horny_cactus's CS:GO/GSI/Buttplug interface](https://sr.ht/~hornycactus/CrotchStimGetOff) for the latest versions of everything and hopefully made running it a bit more simple, adding a small UI as well as the existing CLI app. 
I am not cool enough to think up a CS2 pun with the sheer power of 'Crotch-Stim: Get Off' so I haven't tried.
I've tried to preserve as much of the original readme as possible (all license terms etc persist) but have updated the relevant parts.
This update depends on a patched version of the csgo-gsi crate that I have included as a submodule.

## Important ethical disclaimer

This software is intended for risk-aware, consensual sexual enjoyment by all.
CS2 is a game with matchmaking that can put you in matches with whoever the hell.
Don't use this in matchmaking or generic community servers.
Use it in botmatches, or on servers set up for the purpose of horny and populated by adults who know what they're getting into.

## Usage
To use the CLI:
1. Build this repo.
       - Clone this repository, update the submodules, grab a recent version of Rust, run `cargo build --release`.
2. Run Intiface Central and start the server.
3. Run `target/release/cs2-buttplug-cli.exe`.
       - It'll create default config and script files in the same folder you put the program itself into and give you a chance to edit them before it starts running.
       - The CS2 scripts folder will need to be writable by the executable.
4. Play CS2 and have fun things happen!

To use the UI:
1. Build this repo.
       - Clone this repository, update the submodules, grab a recent version of Rust, run `cargo build --release`.
2. Run Intiface Central and start the server.
3. Run `target/release/cs2-buttplug-ui.exe`.
       - Use the UI to navigate to your CS2 scripts dir (typically `C:\Program Files (x86)\Steam\steamapps\common\Counter-Strike Global Offensive\game\csgo\cfg`)
       - This folder will need to be writable by the executable.
4. Click 'Launch'
5. Play CS2 and have fun things happen!

## Configuration

For the CLI, a file like this will be created in `cs2-buttplug-cli.toml`.

```toml
# specify your Intiface server,
# or leave it out to connect to ws://127.0.0.1:12345/ (the default address of an Intiface install)
buttplug_server_url = 'ws://127.0.0.1:12345/'

# the port number to use for the CS2 integration server.
# can be anything that isn't already in use on your computer
cs_integration_port = 42069

# path to cs2 on your machine - this is the default so if you have it installed elsewhere set that location here.
cs_script_dir = 'C:\\Program Files (x86)\\Steam\\steamapps\\common\\Counter-Strike Global Offensive\\game\\csgo\\cfg'
```

## Scripting

The plugin gets its 'game logic' from a [Rhai](https://schungx.github.io/rhai/) script that is copied into the directory with the executable. 

The default script looks like this:
```rhai
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
```

It'll give you a steadily escalating buzz each time you get kills/assists over the course of a round.

Your script needs to define a function called `handle_update`.
It'll be called with [an `Update` object](https://docs.rs/csgo-gsi/0.3.0/csgo_gsi/update/struct.Update.html) whenever CS:GO sends some new data.
All those optional values are translated into Rhai as either the value itself if it exists or the unit `()` if there's no value.
That's why the example script checks if things are equal to `()`.
All the enums defined by `csgo-gsi` can be `.to_string()`ed and tested against string values, like the default script does for `WeaponState`.
All the maps get translated into Rhai maps; several of those should probably just be arrays in the first place, so go poke the `csgo-gsi` author about that if you need reasonable access to that data.

The only command functions implemented are `vibrate(strength, duration_in_seconds)` and `stop()` but the infrastructure is there to add more.

Feel free to file a github issue for any problems you have and I'll try to take a look.

## License

This software is released under the [Fuck Around and Find Out License version 0.2](https://git.sr.ht/~boringcactus/fafol/tree/master/LICENSE-v0.2.md).
The `csgo-gsi` library that this uses is released under the [Anti-Capitalist Software License version 1.4](https://anticapitalist.software/).

## Changelog

v0.4.0 - 2024-01-22
- Updated for CS2/Buttplug 7+
- Removed the updater and CS2 directory-finding code for the sake of code simplicity.
- Added an intermediate thread that can reprocess events from the Rhai script.
- Added an egui-based UI.

v0.1.0 - 2020-09-23
- write the damn thing
