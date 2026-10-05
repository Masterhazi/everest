# Everest: Handover Notes

Paste this file (or point Claude at it) in the new chat. Repo: github.com/Masterhazi/everest (branch `master`).

## 1. How to work with the user (Hazi Aafrid Baba)
- Talk first, build only after the user says "go". They have repeatedly stopped builds that started too early.
- Start answers with a short, crisp bullet summary; give long detail only if they ask.
- Spell out abbreviations and jargon with a one-line explanation.
- They write in Hinglish (Hindi in Latin script mixed with English). Reply in the same style.
- Test device: Samsung S20 FE 5G. Laptop: Windows, Intel 11th-gen H-series, 16 GB RAM, desktop name `hazi-ultra`.
- Everything must be free (no paid tools), except they might consider Aseprite later.

## 2. The game
"Everest" (working title "Love While It Lasts"), a story by Hazi Aafrid Baba. A portrait, mobile-first, pixel-art survival-climbing game. A grieving man climbs Everest to fulfil his dead friend's 13th-day wish.
- No text-box exposition. No game over: when the player falls, he gets up again.
- Nature is the antagonist.
- Ending: whiteout at the summit, the friend's yellow coat flying, camera pulls back, clouds, audio continues, title "LOVE WHILE IT LASTS / A story by Hazi Aafrid Baba".
- Locked decisions: the coat is found during the first avalanche; the cross is kept (picked up at Base Camp); the friend's voice begins after the coat discovery (subtitles for now, real Hindi voice later); the friend's coat is yellow, the hero's is red.
- Tech: Rust + Bevy 0.16.1, Android APK built by GitHub Actions (cargo-apk). Android uses Vulkan first with OpenGL ES as fallback.
- Must stay very light (target Snapdragon 4 Gen 1 / 450-class phones).

Read `README.md` in the repo for controls, hazards, adaptive difficulty, sound and project layout.

## 2b. State of the build
- Vertical slice works: Base Camp, continuous mountain, 5 tools always visible with no hints, hazards (rockfall, crevasse, icefall, random avalanches, altitude dizziness, night with headlamp), adaptive difficulty, hints after repeated failures, control strip at the bottom 27%, adaptive soundtrack, ending song at the end, app icon.
- The latest APK (Vulkan-first fix) was built and passed the emulator test. Not yet confirmed on the user's S20 FE.
- CI: `.github/workflows/android.yml` publishes the "latest" release with `everest.apk`.

## 3. Decisions made in the last conversation (NOT yet built)
The user said "go-ahead in principle" on all of these; ask for a final "go" before building:
1. **Escape moves** for hazards:
   - **Dive:** sideways dive out of a rockfall lane.
   - **Brace:** contextual Rest, crouch under an overhang or behind the boulder.
   - **Avalanche:** "swim" (keep moving on top of the debris) and "air pocket" (cover face / dig) when buried.
   - **Crevasse:** axe-catch while falling in.
   - No umbrella (dropped).
2. **Wind gusts come back** (the mountain stays one continuous climb). Wind pushes, can force a brace, and builds near exposed traverses.
3. **Cold / frostbite stays out** (extra complexity; gear would fix it anyway).
4. Mountain stays continuous; falls are resolved by physics (slide until terrain stops him).

## 4. Animation plan (the biggest quality gap)
Current character sprites are cut from an AI concept sheet and flicker. Plan: make a consistent animated character with free tools.
- **Blender** (3D), **Mixamo** (free Adobe animation library), **LibreSprite** (free pixel editor), and the **Blender MCP** add-on (so Claude can drive Blender with Python).
- Render low-resolution frames on a fixed grid (e.g. 32x48), convert to pixel art, touch up in LibreSprite.
- Keep `assets/sprites/hero.png` as a 5x9 grid of 96x88 cells so the code needs no changes.
- Mixamo has walk, run, idle, fall, get-up, etc. Ice-axe swing, self-arrest and rope clipping are NOT in Mixamo; pose them in Blender by script.

### Setup status on the user's laptop
- Blender 5.2.2 LTS installed. The MCP add-on (v1.8) is installed and shows "Connected on port 9876". DONE.
- Still to confirm: `uv` installed (`uv --version` in PowerShell), `blender` entry in Claude desktop config (`"mcpServers": {"blender": {"command": "uvx", "args": ["blender-mcp"]}}`), Mixamo Adobe account, LibreSprite installed.
- The old chat could not be linked to the laptop ("Link to this computer" did not appear). In the new chat: start a Local session on `hazi-ultra` with a folder such as `C:\Users\hajia\Claude`, clone the repo, then test a small Blender command (e.g. create or rotate a cube).

## 5. Roadmap (suggested order)
1. Confirm the latest APK runs on the S20 FE.
2. Get the laptop link + Blender MCP working (test command).
3. Build the animation pipeline (hero walk, climb, axe, slide, fall, rise, rest).
4. Escape moves + wind gusts (section 3).
5. Juice: screen shake, particles, camera feel.
6. Menus: title screen, pause, settings, credits (CC BY "ooh" sample needs in-game credit, see `CREDITS.md`), save progress.
7. Real Hindi voice lines for the friend; optionally a human-composed memory theme.
8. Low-end phone testing (Firebase Test Lab), 30 fps cap, internal resolution idea 180x320.
9. Play Store assets (icon, screenshots, description, privacy policy).

## 6. Audio and rights notes
- Music is generated by `tools/gen_music.py` (drone, bowls, tension, night shimmer, lament built from three free voice samples in `tools/samples/`).
- "Ashchhe Jamai Digombor" (Parvathy Baul Project, SVF Music): the user says they hold the rights. A 95 s filtered excerpt (`song_distant.ogg`) plays at the end.
- Samples: Tibetan chant (CC0, nemaavla), female "ooh" (CC BY 3.0, TheScarlettWitch89, attribution required), kulning (CC0, ZebNilsson).

## 7. Handy dev commands
- Desktop run: `cargo run --release`
- Headless playtest (Linux): `tools/playtest.sh shots 150`
- Build status / APK: GitHub repo, Actions tab, and Releases, "latest".
