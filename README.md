# cs2-buttplug: The bomb has been planted

I've updated [horny_cactus's CS:GO/GSI/Buttplug interface](https://sr.ht/~hornycactus/CrotchStimGetOff) for the latest versions of everything and hopefully made running it a bit more simple, adding a small UI as well as the existing CLI app. 
I am not cool enough to think up a CS2 pun with the sheer power of 'Crotch-Stim: Get Off' so I haven't tried.
I've tried to preserve as much of the original readme as possible (all license terms etc persist) but have updated the relevant parts.
This update depends on a patched version of the csgo-gsi crate that I have included as a submodule.

![csbp-0 11](https://github.com/user-attachments/assets/5da19cc1-d59f-46b7-9b22-b032ca15ffe0)

## Will it get me banned?

cs2-buttplug takes advantage of the CS2 Game State Integration (GSI) used for external display of scores and other information at tournaments and live events. GSI is an official interface provided by Valve and does not provide any information that could allow a competitive advantage. While there's no reason to expect that running this software alone will result in your being banned from CS2, Valve have ultimate discretion over what they allow to interact with their game.

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

let steam_id = "";

fn handle_update(update) {
    if update.player != () {
        if steam_id == "" {
            steam_id = update.player.steam_id;
        }
        if steam_id == update.player.steam_id {
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
}
```

It'll give you a steadily escalating buzz each time you get kills/assists over the course of a round.

Your script needs to define a function called `handle_update`.
It'll be called with [an `Update` object](https://docs.rs/csgo-gsi/0.3.0/csgo_gsi/update/struct.Update.html) whenever CS:GO sends some new data.
All those optional values are translated into Rhai as either the value itself if it exists or the unit `()` if there's no value.
That's why the example script checks if things are equal to `()`.
All the enums defined by `csgo-gsi` can be `.to_string()`ed and tested against string values, like the default script does for `WeaponState`.
All the maps get translated into Rhai maps; several of those should probably just be arrays in the first place, so go poke the `csgo-gsi` author about that if you need reasonable access to that data.

Command functions currently implemented are:
- `vibrate(strength, duration in seconds)`
- `vibrate_index(speed, duration in seconds, index of motor)`
- `stop()`
- `linear(duration in milliseconds, position in percent from 0.0 to 1.0)`
- `sleep(duration in seconds);`

Feel free to file a github issue for any problems you have and I'll try to take a look.

## License

This software is released under the [Fuck Around and Find Out License version 0.2](https://git.sr.ht/~boringcactus/fafol/tree/master/LICENSE-v0.2.md).
The `csgo-gsi` library that this uses is released under the [Anti-Capitalist Software License version 1.4](https://anticapitalist.software/).

## Changelog
v0.11.0 - 2025-06-25
- Resolved a bug where the application would crash on relaunching.
- Some improvements to live data view in UI.
- Multiple scripts can now be stored in the 'scripts' subdirectory and switched between in the UI on the fly.
- The handle_update main script folder now has an is_player variable handed to it for convenience. When is_player is false, the CS session is spectating (the active steam id is not the current player).
- Default script for UI is now generated into scripts subdirectory.

v0.10.0 - 2025-01-29
- Some GSI update values exposed in a UI panel for monitoring.
- Initial support for linear devices courtesty of DasPauluteli ([#5](https://github.com/gloss-click/cs2-buttplug/pull/5))
- Removed dependency on fehler-throws.

v0.9.0 - 2024-08-06
- Increase RHAI operation count.
- Increase GSI heartbeart to 1s
- Limit vibrateindex calls to the available index range
- Check steam id in default RHAI script so that spectating people who gets kills won't skew your count

v0.8.0 - 2024-06-02
- Add log panel to gui
- List devices in GUI and allow enabling and disabling them 
- Automatically launch client on start 
- Break up core nest of futures a little bit for more interactivity in future

v0.7.0 - 2024-04-15
- Resolves an issue where device discovery events could arrive before we were monitoring for them.

v0.6.0 - 2024-03-14
- Updates from CS faster to be more responsive and allows multiple time periods to overlap, as well as adding basic support for devices with multiple facets.

v0.5.0 - 2024-02-01
- Swaps the gotham web server out for the warp web server in csgo-gsi, as gotham did not like being restarted once it had started to process requests.
- Code complexity reduction too, probably at the cost of resiliance against bad requests.

v0.4.0 - 2024-01-22
- Updated for CS2/Buttplug 7+
- Removed the updater and CS2 directory-finding code for the sake of code simplicity.
- Added an intermediate thread that can reprocess events from the Rhai script.
- Added an egui-based UI.

v0.1.0 - 2020-09-23
- write the damn thing
