# EVEREST — vertical slice

*Love While It Lasts* · a story by Hazi Aafrid Baba

Portrait, touch-first, Rust + Bevy 0.16.

**What's in this slice:** Base Camp (Day 1) → first ascent (rock, then ice) → the exposed windy traverse → the first avalanche → the friend's yellow coat → the friend's voice → the first checkpoint (Day 3) → fade out.

---

## Controls

| Where | Phone | Desktop (for testing) |
|---|---|---|
| Move / climb | Left thumb: joystick (touch anywhere in the lower-left) | Arrows or WASD |
| Ice axe | Right side: the axe appears at ice walls | Space |
| Rope | Right side: appears at the anchor | R |
| Dig | Right side: appears when buried, and at the coat | E |
| Rest | Right side: appears when he's tired | Q |

Only the tools that make sense right now are shown.

## What the player does

- **Rock walls:** push up. It costs stamina.
- **Ice walls:** tap the **axe** to plant it, then push up to pull. The rhythm is plant → pull → recover. Without a planted axe he slowly slides down the ice.
- **Stamina is never shown as a bar.** It comes through his breathing, the dark vignette at the screen edges, and his pace. If it runs out on a wall, he falls to the bottom. The screen flashes, he lies there, then gets up again. There's no game over. Every second fall he mutters *"Ek aur climb."*
- **Exposed traverse (Day 1 → 2):** wind gusts are announced by the sound and the snow picking up. If he's unroped and moving when a gust hits, it knocks him back. The player can clip the **rope** to the anchor, which makes him slower but safe, or cross unroped and stop still during each gust. Either choice works.
- **Avalanche:** a *whumpf*, sudden silence, and a crack in the slope, then about 2 seconds before it arrives. If he reaches the **leeward side of the boulder**, he's sheltered. Anywhere else, he's swept away and buried: the screen goes dark, you hear his heartbeat, and the player **digs** out. Either way, the churned snow uncovers a yellow sleeve.
- **The coat:** he stops at it. Dig four times, he kneels, the wind drops, and he takes the coat. It's rolled on top of his pack for the rest of the game. Then the friend's voice: *"Jab toofan aaye toh bhaagte nahi… bachte hai."*
- **Checkpoint:** he sits beneath the prayer flags, the sky turns to sunset, **DAY 3** appears, and the friend tells a memory.
- If the player stops, nothing happens and the game waits.

## Run it

```bash
# desktop (fastest way to iterate)
cargo run --release
```

### On your phone, in the browser (easiest)

```bash
rustup target add wasm32-unknown-unknown
cargo install --locked trunk
trunk serve --release --address 0.0.0.0
```

With your phone on the same Wi-Fi, open `http://<your-computer's-IP>:8080`. Use **Add to Home Screen** to get it full screen. `trunk build --release` puts a static site in `dist/` that you can host anywhere (GitHub Pages, Netlify, itch.io).
Browsers block sound until the first touch, so the first tap on the joystick also starts the audio.

### Android APK (built by GitHub, free)

This repo includes `.github/workflows/android.yml`. Every push to `main` makes GitHub build a signed APK
(~15 min first time, faster after) and publish it at:

`https://github.com/<you>/<repo>/releases/latest` → tap **everest.apk** on your phone → install
(allow "Install unknown apps" for your browser when Android asks).

You can also start a build by hand: repo → **Actions** → **Android APK** → **Run workflow**.
If a build fails, the red ✗ in Actions shows the log — paste it to Claude.

Building locally instead needs the Android SDK + NDK:
`rustup target add aarch64-linux-android && cargo install cargo-apk && cargo apk run --release --lib`

## Project layout

```
src/lib.rs        app setup, system order
src/level.rs      the mountain: ledges, walls, props (all positions are here)
src/hero.rs       movement, climbing, axe rhythm, stamina, falls
src/story.rs      the beats: cross, rope, gusts, avalanche, coat, voice, checkpoint
src/fx.rs         camera, parallax sky, snow, vignette/darkness, captions, audio mix
src/controls.rs   touch joystick, contextual tool buttons, keyboard, test autopilot
tools/slice.py    cuts the concept sheet into sprites (and recolours the friend's coat)
tools/gen_assets.py  placeholder sky, rock face, avalanche, and all sounds
tools/playtest.sh automated play-through with screenshots (Linux, headless)
```

## Placeholder vs final

- **Sprites** are cut from the AI concept sheet. They're fine for statics (tent, coat, cross, icons), but the character frames aren't consistent, so animations flicker a little. The final character should be redrawn on a fixed grid (e.g. 32×48, transparent, one palette). `assets/sprites/hero.png` is a 5×9 grid of 96×88 cells, and keeping that layout means no code changes.
- **Audio** is procedurally generated: wind, gusts, breathing, heartbeat, whumpf, rumble, axe, crunch, and a soft drone for memory moments. *Asche Jamai Digambar* is **not** included. Drop a licensed recording into `assets/audio/` later.
- **Friend's voice** is shown as subtitles for now. Real Hindi voice lines go in later.
- **Font** is Bevy's built-in default. Swap in a pixel font via `TextFont`.

## Dev: automated play-test

```bash
EVERY=2 tools/playtest.sh shots 150                    # plays through, screenshot every 2 s
tools/playtest.sh shots 90 EVEREST_AUTOPLAY_SHELTER=1   # take shelter at the boulder instead
tools/playtest.sh shots 90 EVEREST_AUTOPLAY_NOROPE=1    # cross the traverse unroped
```
