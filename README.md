# Crotch-Stim: Get Off

what if CS:GO was horny?

## configuration

```toml
# you can put this in to connect to an Intiface Desktop server,
# or leave it out to use the embedded server (the default)
buttplug_server_url = 'ws://127.0.0.1:12345/'

# the port number to use for the CS:GO integration server.
# can be anything that isn't already in use on your computer
csgo_integration_port = 42069
```

## scripting

there's a lot that CS:GO gives you.
and i'm bad at user interface design.
so you can control how everything interacts via a script that handles updates.

the default script looks like this:
```rhai
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
```

this is a script in the [Rhai](https://schungx.github.io/rhai/) programming language.
it looks through the player's weapons, finds the one that they're holding, checks to see how much ammo is left in the clip (or magazine, i don't know guns, that's CS:GO's term not mine do not @ me), and vibrates based on how much of the clip is gone.
you can write your own and put it in the `crotch-stim-get-off.rhai` file, or send one you wrote to your friends.
i might start putting together examples at some point so if you write something you like email `me@cactus.sexy` and i'll think about it.

your script needs to define a function called `handle_update`.
it'll be called with [an `Update` object](https://docs.rs/csgo-gsi/0.3.0/csgo_gsi/update/struct.Update.html) whenever CS:GO sends some new data.
all those optional values are translated into Rhai as either the value itself if it exists or the unit `()` if there's no value.
that's why the example script checks if things are equal to `()`.
all the enums defined by `csgo-gsi` can be `.to_string()`ed and tested against string values, like the default script does for `WeaponState`.
all the maps get translated into Rhai maps; several of those should probably just be arrays in the first place, so go poke the `csgo-gsi` author about that if you need reasonable access to that data.

currently, because i am extraordinarily lazy, there is only one way to interact with your sex toy itself.
the `vibrate` function is available for your script to call, and if you pass in a float between 0 and 1 it will send that as a basic vibrate command to every device that it can find.
if you want support for multi-parameter vibration, device-specific commands, or other types of commands, poke me and i'll take a look.

i don't trust any of this to make sense to literally any person ever besides me, so reach out to me by twitter DMs (@horny_cactus) or by email (`me@cactus.sexy`) if something's confusing.

## license

this software is released under the [Fuck Around and Find Out License version 0.2](https://git.sr.ht/~boringcactus/fafol/tree/master/LICENSE-v0.2.md).
the `csgo-gsi` library that this uses is released under the [Anti-Capitalist Software License version 1.4](https://anticapitalist.software/).

## changelog

v0.1.0 - pending
- write the damn thing
