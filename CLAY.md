# Clay — project state

Clay is a fork of [Zed](https://github.com/zed-industries/zed) that combines an inline browser, a unified AI terminal, and selected tooling from other Zed forks.

**This file is the source of truth for "where are we".** Start with the **Roadmap and scope tracker** immediately below: it is the live status of every wanted feature. Everything after it is the historical record, kept for rationale rather than status. Update both after each meaningful unit of work. It exists so that work can resume cleanly across sessions without reconstructing context from a lost conversation.

Full design rationale lives in the plan file: `C:\Users\Louis\.claude\plans\binary-wandering-cray.md`.

---

# Roadmap and scope tracker

Status at a glance. Everything below the horizontal rule that closes this section is the
historical record — read that for *why* a thing was done, not for *whether* it is done.

**Keep this table current.** It is the first thing anyone reads, including a future session
picking up cold.

Legend: **Done** · **Active** (in progress now) · **Next** (immediately after Active) ·
**Planned** · **Deferred** (wanted, not scheduled) · **Cut** (decided against)

## Milestones, in priority order

| # | Milestone | Status |
|---|---|---|
| **M0** | Fork, rebrand, isolation from stock Zed | **Done** — committed |
| **M1** | Windows D3D11 surface compositing in GPUI | **Done** — committed |
| **M2** | Inline browser (from Glass) — a real page renders in a pane | **Done** — gate passed 2026-09-03 |
| **M3** | Finish the rebrand: remaining Zed branding in the UI | **Mostly done** — app chrome swept; `ZED_*` in `script/`/`.github/` and the tagline outstanding |
| **M4** | Clay icons everywhere — app, tray, installer, in-app | **Mostly done** — app/tray icons and the in-app logo shipped; macOS `.icns`, packaging and the AI feature glyphs outstanding |
| **M5** | AI account switcher | **Planned** |
| **M6** | Continue-after-credit-reset scheduling | **Planned** |
| **M7** | Unified AI terminal | **Planned** |
| **M8** | Rest of the Lathe port — `pr_ui`, workspace groups | **Planned** |
| **M9** | Integration between browser, terminal and editor | **Planned** — deliberately last |

This ordering supersedes the original Phase 3/4/5 plan further down the file. What changed: the
rebrand and icon work is pulled to the front, and the AI account switcher is pulled out of the
Lathe milestone and ahead of the AI terminal, because it is wanted soonest.

## M3 — Finish the rebrand

The isolation rebrand (M0) covered the names that matter for *not colliding with a stock Zed
install*: data directories, identifiers, env vars, URL scheme, binary name. It deliberately did
not sweep user-facing display strings, and those are the visible leftovers.

| Item | Where | Status |
|---|---|---|
| App menu titled "Zed", with "About Zed" and "Quit Zed" | `crates/zed/src/zed/app_menus.rs` | **Done** — now uses `ReleaseChannel::display_name()`, so a dev build reads "Clay Dev" and stable reads "Clay", matching upstream's convention of distinguishing channels |
| "Welcome back to Zed" on the empty-workspace screen | `crates/workspace/src/welcome.rs` | **Done** — verified on screen |
| "Welcome to Zed" in onboarding | `crates/onboarding/src/onboarding.rs` | **Done** |
| About-window title, launch-failure dialog and notification, CLI system-specs header | `crates/zed/src/zed.rs`, `crates/zed/src/main.rs` | **Done** |
| Update-available and xdg-portal notifications, collab "unshared project" / "window outside of Zed" labels | `crates/workspace/src/notifications.rs`, `pane_group.rs` | **Done** |
| macOS single-instance banner and the move-to-Applications flow | `mac_only_instance.rs`, `move_to_applications.rs` | **Done** — string-only, not verifiable from Windows |
| "Setup Zed REPL for …" tooltip | `crates/zed/src/zed/quick_action_bar/repl_menu.rs` | **Done** |
| "Welcome to Zed AI / Pro / Business / VIP / Student" | `crates/ai_onboarding/src/ai_onboarding.rs`, 7 sites | **Out of scope** — service names, per the decision above |
| Tagline "The editor for what's next" | `crates/onboarding/src/onboarding.rs`, welcome screen | **Open, needs Louis** — this is Zed's slogan and still shows under "Welcome back to Clay". Left rather than invented; needs his words or a decision to drop the line |
| `ZED_*` names in `script/` and `.github/` | ~40 names; packaging is inconsistent with the Rust code, which now expects `CLAY_*` | Open |
| `GLASS_CEF_DEBUG` should become `CLAY_CEF_DEBUG` | `crates/browser` | Open |
| Remaining `Zed` literals | What is left is diagnostics and log context (`.context("Handshake before Zed spawn")`), telemetry property descriptions, the HTTP User-Agent, test fixtures, and `ui` component previews. None is app chrome; all deliberately untouched | Open by choice |

Implementation note: display strings now interpolate `paths::APP_NAME` rather than hardcoding
"Clay", so a future rename is one constant. This meant adding `paths` as a direct dependency of
`workspace` and `onboarding`, which reached `APP_NAME` through neither (`util::paths` is a
different module from the `paths` crate).

**Decided: app chrome only.** Menus, welcome screens, window and app titles, quit/about. The
names of Zed-operated *services* — "Zed AI", "Zed Pro", "Zed Business", the Zed-hosted model
providers — are deliberately left alone, because Clay would still be talking to Zed when it uses
them, and renaming them would misrepresent whose service it is. Revisit only if the open question
of whether Clay should talk to `zed.dev` at all is answered with "no".

Also out of scope for the same reason: `zed.dev` URLs, the "Zed Repository"/"Zed Twitter"/"Email
Us" feedback links, the internal `zed` Cargo package, and the `IconName::Zed*` variants that name
an asset file. A blind find-and-replace would break the build and misname third-party services.

## M4 — Clay icons

The masters are `clay_icon.svg` and `clay_icon.png` at the repo root, both added by Louis. The
SVG is a **vectorised pixel-art trace** of the 64×64 PNG: 8 paths of 1×1 rectangles, 8 hardcoded
fills, `shape-rendering="crispEdges"`.

Two consequences worth knowing before touching this again:

- **The mark is multi-colour, so it cannot go through `Vector`.** `Vector` renders via gpui's
  `svg()`, which masks the file and tints it with a single theme colour — the mark would flatten
  to a silhouette. The places that show the logo use `img()` instead, which goes through
  `usvg`/`resvg` and keeps the colours. gpui's `img()` does accept `.svg`.
- **Every icon size is an exact multiple or divisor of 64**, so the raster can be scaled without
  inventing detail. `_refs/gen_icons.py` upscales with NEAREST (matching the SVG's `crispEdges`,
  so it is equivalent to rasterising the vector) and downscales 16px/32px with LANCZOS, where
  legibility beats crisp edges.

| Item | Where | Status |
|---|---|---|
| Windows app and tray icon | `crates/zed/resources/windows/app-icon{,-dev,-preview,-nightly}.ico`, each 16/32/64/128/256 | **Done** — verified by extracting the icon from `clay.exe` |
| PNG app icons | `crates/zed/resources/app-icon*.png` — 8 files, 512 and 1024 | **Done** |
| In-app logo, welcome screen and onboarding | `crates/workspace/src/welcome.rs`, `crates/onboarding/src/onboarding.rs` | **Done** — full colour, verified on screen |
| `VectorName::ZedLogo` → `ClayLogo`, asset `images/clay_logo.svg` | `crates/ui/src/components/image.rs` | **Done** — `zed_logo.svg` deleted |
| macOS document icon | `crates/zed/resources/Document.icns` | Open — this Pillow build cannot write ICNS, so it needs `iconutil` on a Mac or another tool |
| Installer and desktop entry | `crates/zed/resources/windows/zed.iss`, `zed.desktop.in`, `flatpak/`, `snap/` | Open — packaging, alongside the `ZED_*` renames |
| AI feature glyphs | `assets/icons/ai_zed.svg`, `zed_agent*.svg`, `zed_assistant.svg`, `zed_predict*.svg` | Open — these are small monochrome feature icons, not the app logo, and some name Zed features rather than Clay's. Needs a decision per icon |
| Channel differentiation | All four channels now share one mark | Open — Zed's dev/preview/nightly icons were differently coloured, so builds are no longer distinguishable by icon |

Build-script fix needed along the way: `cargo:rerun-if-changed` for the icon lived only in the
X11 path, and `icon_path()` was gated to Linux/FreeBSD despite containing a `#[cfg(windows)]`
branch. Changing the `.ico` therefore did not rebuild the embedded resource on Windows — the exe
kept Zed's mark until the gate was widened.

## M5 — AI account switcher

Multi-account AI sign-in: hold several accounts and switch between them without
re-authenticating.

**Decided: do both, behind one switcher UI.** A single account picker in Clay that switches the
account for Clay's own agent *and* for the Claude Code CLI, so accounts are managed in one place.
That means porting Lathe's crate for the former and adopting cc-switch's credential rewriting for
the latter, behind a shared front end.

The two sources:

- **Lathe's `ai_accounts`** (~1.4k lines, at `_refs/lathe/crates/ai_accounts`) — per-agent account
  index, default-account resolution, on-disk persistence under `accounts_root()`, per-account
  state, `AiAccountsSettings::resolve_account`. Nothing equivalent upstream. Lathe also has a
  multi-account **collab** switcher, which lives in its `client`/`title_bar` edits rather than in
  the crate.
- **cc-switch** — the Claude Code Multi-Account Switcher, which changes the active account by
  rewriting Claude Code's own credential and `oauthList` state.

These solve overlapping but different problems: Lathe's crate switches which account *Clay's own
agent* uses, while cc-switch switches which account the *Claude Code CLI* uses. Resolve before
starting.

## M6 — Continue after credit limits reset

Schedule a follow-up message to be delivered once usage limits reset, so a long-running agent
task resumes on its own.

Requirements as stated:

- **Fully automatic by default** — no user input at the moment of reset.
- **A manual option too**, so the user can drive it for their own automation.

Not yet designed. Open questions: how the reset time is discovered (parsed from an API error,
read from a response header, or computed from a known window); whether the follow-up is a stored
prompt replayed into the same thread or a fresh turn; and how it interacts with M5, since
switching accounts is the other obvious response to hitting a limit.

## M7 — Unified AI terminal

Design already recorded under "Scope: what Clay adds" below, unchanged: one block timeline, three
block kinds (shell command bounded by OSC 133 marks, agent turn, in-process app command), with
the **agent as the default input and `!` as the explicit shell escape**.

## M8 — Rest of the Lathe port

`pr_ui` (~6.7k lines — real PR review, since upstream Zed is link-out only) and workspace groups
(~1k self-contained lines, the cheapest item and a good first port). Re-measure against Zed at
Lathe's sync point first: Lathe is ~7 weeks behind our base, so diffs overstate its change set.

## Open gaps carried forward — wanted, not yet scheduled

| Item | Notes |
|---|---|
| Browser `SerializableItem` | Browser *tabs* survive a restart but the *pane* does not. Louis asked for this explicitly. Use `workspace::register_serializable_item::<BrowserView>`; needs no changes to `crates/workspace`. |
| Browser right-click menus | Stubbed, with `TODO`s in `tab_strip.rs` and `bookmarks.rs`. Glass dispatched on an item index from a single AppKit callback; `ui::ContextMenu` takes a handler per entry and needs menu state plus a dismiss subscription on the owning view. |
| `ctrl-alt-*` and chord keybindings do not fire | Almost certainly AltGr on a UK layout. The command palette is the reliable route meanwhile. |
| Misleading "scene too large" error prefix | It wraps every renderer batch failure, not just size problems. |

| Deferred from Lathe | Theme customizer (135+ colours, cheap and self-contained) and `git_graph`, which overlaps upstream's `git_ui/src/git_graph.rs`. |
| Cut from Lathe | `mobile_dev` (Expo/React Native) and `aws_dev`. |
| Undecided | Whether Clay should talk to `zed.dev` at all — collab, telemetry and docs endpoints are untouched. |

---

## Historical record: Phase 0 (fork, build, baseline)

**COMPLETE.** See "Phase 0 COMPLETE" below for gate evidence; the table here is the per-item record.

| Item | State |
|---|---|
| Fork created | Done — https://github.com/LouisHitchcock/Clay (public, personal account `LouisHitchcock`) |
| Local clone | Done — `C:\Users\Louis\Desktop\Code\Clay`, full history, 39,804 commits, HEAD `c3cf80c` (2026-09-02), 520 MB |
| Remotes | Done — `origin` → `LouisHitchcock/Clay`; `upstream` → `zed-industries/zed` with push URL set to `DISABLED_use_origin` to prevent accidental pushes upstream. `main` tracks `origin/main`. |
| Toolchain verified | Done — Rust 1.97.1 (matches `rust-toolchain.toml`), MSVC BuildTools 2022 x64, cmake 3.29.2, ninja 1.12.0 |
| Baseline release build | **PASSED** — `cargo build --release -p zed` exit 0, 19m19s for the final crate, produced `target/release/zed.exe` (422 MB with debuginfo). **This was the Phase 0 gate: the Windows toolchain is proven.** Note this binary is *unmodified Zed*, built before the isolation changes. |
| Rebrand + isolation | **Done and committed** on branch `clay/rebrand-and-isolate`. Unpushed. |
| Reference checkouts | Moved into the repo at `Clay\_refs\` (was `Code\_refs\`) to keep Louis's `Code\` directory clean. Ignored via **`.git/info/exclude`**, deliberately *not* `.gitignore`, so the upstream-tracked file stays pristine and cannot cause merge conflicts. Verified invisible to `git status`. |

### Isolation checklist — current state

| # | Item | State |
|---|---|---|
| 1 | `APP_NAME` → `"Clay"` (`paths.rs:18`) | **Done.** Isolates all user data/config/cache/state/logs in one edit. |
| 2 | Project dir `.zed/` → `.clay/` (`paths.rs:488,499,506,529`) | **Done.** No `.zed/` fallback, per decision. |
| 3 | `.zed_server` / `.zed_wsl_server` → `.clay_*` (`paths.rs:71,79`) | **Done.** |
| 4 | Log filenames | **Done** — derive from `APP_NAME`, so now `Clay.log`. |
| 5 | `app_identifier()` → `Clay-Editor-*`; `app_id()` → `io.github.louishitchcock.Clay*` | **Done.** Also isolates the Windows single-instance mutex + named pipe for free. Display names → "Clay", "Clay Dev", etc. |
| 6 | `ZED_VARIABLE_NAME_PREFIX` → `"CLAY_"` (`task.rs:254`) | **Done.** One line renames the whole task-variable surface. |
| 7 | Runtime `ZED_*` env vars → `CLAY_*` | **Done** — 152 string literals across all Rust sources, including build scripts, so producers and consumers stay consistent. Zero `"ZED_` literals remain. |
| 8 | URL scheme `zed://` / `zed-cli://` → `clay://` / `clay-cli://` | **Done** — all 91 occurrences across 20 files. |
| 9 | **Auto-update disabled** (`auto_update.rs:276-282`) | **Done.** `poll_for_updates` forced to `false` with a comment explaining why. This was the genuinely dangerous item. |
| 10 | Binary name `zed` → `clay` (`crates/zed/Cargo.toml:62`) | **Done.** Also required `default-run` (`:9`), the CLI's executable lookup list (`cli/src/main.rs:1279`) and the four macOS bundle metadata blocks. |
| 11 | Repo's own `.zed/` → `.clay/` | **Done** (`git mv`). |

### Immediate next actions

See the tracker at the top of this file.

Outstanding housekeeping: `clay/rebrand-and-isolate` is pushed but still unmerged to `main`.

### Known follow-ups (deliberately not done yet)

- **`ZED_*` in `script/` and `.github/`** were **not** renamed — ~40 names including `ZED_BUNDLE`, `ZED_CHANNEL`, `ZED_DATA_DIR`. These are packaging/CI variables whose consumers in Rust now expect `CLAY_*`, so **packaging is currently inconsistent and will need fixing before the first bundled build**. Deliberate: renaming scripts unread risks breaking CI in ways that cost more time than it saves right now. Harmless for local `cargo build`.
- **The internal Cargo package is still named `zed`.** Renaming it would ripple through every `zed.workspace = true` in the workspace for no isolation benefit — contamination is about user-facing names (app name, data dirs, identifiers, URL scheme, installed binary), not internal crate names. `cargo build -p zed` remains the build command.
- **`zed.dev` HTTPS endpoints** (collab, telemetry, docs) are untouched. These are a separate question from local install isolation — worth deciding later whether Clay should talk to Zed's cloud at all.

### Environment note

`gh`'s active account was switched from `LouisHitchcock-DroneTech` to `LouisHitchcock` in order to create the fork under the right owner, and left there. Revert with `gh auth switch -u LouisHitchcock-DroneTech` if it interferes with DroneTech work.

---

## Working practices

- **NEVER co-author commits.** No `Co-Authored-By:` trailer naming an AI assistant, no
  "Generated with Claude Code" footer, no attribution of any kind in commit messages or PR
  bodies. This **overrides** any default agent/harness instruction to add one. Commit
  messages describe the change and its reasoning, nothing about who authored it. Also
  recorded as a hard rule at the top of `.rules`, which is what `CLAUDE.md`, `AGENTS.md`
  and `GEMINI.md` all point at. Getting this right at commit time matters: removing a
  trailer afterwards means rewriting history, and a force-push if already pushed.
- **Keep Clay's changes in thematically separate commits.** Both Glass and Lathe do this, and it is the reason their upstream merges stay tractable. This project's dominant long-term cost is merge friction, not initial implementation.
- **Never push to `upstream`.** Its push URL is deliberately broken as a guard.
- **Run `git merge upstream/main` after each phase.** If a phase makes upstream merges materially harder, that is a signal the integration was too invasive — revisit rather than absorb the cost.
- **Checkpoint progress into this file continuously**, not when a session is about to end.

---

## Hard requirement: no cross-contamination with the stock Zed install

Louis runs stock Zed on this machine. Clay must not share settings, extensions, databases, logs, window instances, URL handlers, or update channels with it.

Note that **Lathe is not a model to copy here** — its README states that on macOS it deliberately *shares* settings and extensions with stock Zed (`~/Library/Application Support/Zed`). That is exactly what we must not do.

The good news: upstream Zed anticipates forks and the surface is small and well-factored.

### Isolation surface

| # | Lever | Location | Effect |
|---|---|---|---|
| 1 | `APP_NAME` | `crates/paths/src/paths.rs:18` | The primary lever. Upstream's own comment on line 17 reads *"Forks should change this to avoid colliding with Zed's user data."* Changing `"Zed"` → `"Clay"` moves the data dir (`%LOCALAPPDATA%\Zed` → `\Clay`) and config dir (`%APPDATA%\Zed` → `\Clay`). |
| 2 | `app_id()` / `app_identifier()` | `crates/release_channel/src/lib.rs:45,228-233` | `dev.zed.Zed{,-Dev,-Nightly,-Preview}` → Clay equivalents. **This also isolates the Windows single-instance machinery for free** — see #3. |
| 3 | Single-instance mutex + named pipe | `crates/zed/src/zed/windows_only_instance.rs:34,67` | Both derive their names from `app_identifier()`: `{app_identifier()}-Instance-Mutex` and `\\.\pipe\{app_identifier()}-Named-Pipe`. So fixing #2 automatically prevents launching Clay from focusing stock Zed's window. Verify by running both at once. |
| 4 | URL scheme | `crates/zed/src/main.rs:1809-1810`, `crates/zed/src/zed.rs:1282-1292`, `crates/cli/src/main.rs:34` | `zed://` and `zed-cli://` → `clay://`. Scheme registration must not steal `zed://` links from stock Zed. |
| 5 | **Auto-update** | `crates/auto_update/src/auto_update.rs:354-356` | **Highest-risk item.** Dev/Nightly channels currently point at `github.com/zed-industries/zed`. If left as-is, Clay's updater would download and install *stock Zed over Clay*. Must be repointed at Clay's releases or disabled outright before any packaged build is distributed or run. |
| 6 | Log file names | `crates/paths/src/paths.rs:245,251` | `Zed.log` / `Zed.log.old` → Clay equivalents. Follows from #1 for location, but the filenames are separate. |
| 7 | CLI binary name | `crates/cli` | The `zed` command → `clay`, so it does not shadow a stock Zed CLI on `PATH`. |
| 8 | Remote server dirs | `crates/paths/src/paths.rs:71,79` | `.zed_server` / `.zed_wsl_server` on SSH/WSL hosts. Rename to avoid clashing with stock Zed's remote server on shared hosts. |

### Resolved: project-local directory becomes `.clay/` — total isolation

**Louis's decision: Clay is to be entirely separate from stock Zed. No shared state of any kind.**

So `.zed/` → `.clay/` (`.clay/settings.json`, `.clay/tasks.json`, `.clay/debug.json`), with **no fallback to `.zed/`**. Clay will ignore per-project config that stock Zed wrote; that is the accepted cost of total separation.

Practical consequences to keep in mind:

- Opening an existing project in Clay will not pick up its committed `.zed/settings.json`. Migration is a manual copy. Worth a note in the README later, and possibly a one-off "import from `.zed/`" command — but as an *explicit user action*, never an automatic read.
- Clay's own repo has a `.zed/` directory (inherited from upstream) that should be renamed to `.clay/` so Clay dogfoods its own config.
- The same rule applies to the remote-server directories in item 8 below: `.zed_server` / `.zed_wsl_server` → `.clay_server` / `.clay_wsl_server`.

<details>
<summary>Original analysis (superseded by the decision above)</summary>

### Open decision: project-local `.zed/` directory

`crates/paths/src/paths.rs:486-529` defines the per-project config folder — `.zed/settings.json`, `.zed/tasks.json`, `.zed/debug.json`.

This is a genuine judgement call rather than a bug:

- **Keep reading `.zed/`** — Clay picks up per-project settings that stock Zed (and teammates, and committed repo config) already wrote. Convenient, and arguably correct since these are *project* settings rather than *user* settings. But it is shared state, and a Clay-only setting written there would confuse stock Zed.
- **Switch to `.clay/`** — total isolation, but Clay ignores every existing project's committed Zed config, which is a real ergonomic loss.
- **Read both, preferring `.clay/`** — best of both; costs a little complexity in the settings loader.

Recommendation: **read both, prefer `.clay/`**. Needs Louis's sign-off before implementing.

</details>

### Isolation escape hatches useful for testing

- `ZED_STATELESS` env var (`crates/zed_env_vars/src/`) — runs without persisted state.
- `paths::set_custom_data_dir()` (`crates/paths/src/paths.rs:105`) — redirect the data dir explicitly; must be called before `data_dir()`/`config_dir()` are first read.

---

## Scope: what Clay adds

### 1. Inline browser (from Glass)

Ported from [`Glass-HQ/Glass`](https://github.com/Glass-HQ/Glass), whose `crates/browser` (~11.9k lines) embeds Chromium via CEF (`tauri-apps/cef-rs`, tag `cef-v145.6.1+145.0.28`). Reference checkout: `C:\Users\Louis\Desktop\Code\OLD\Glass`.

Verified facts:

- **`BrowserView` already implements `workspace::Item`** (`browser_view.rs:1291`), as does a separate `BrowserPaneItem` (`:1250`), plus `Focusable` (`:1100`) and `EntityInputHandler` (`:1106`). It is already a pane item, so it can be a browser tab beside editor tabs. It also registers with `workspace_modes::ModeViewRegistry` (`browser.rs:255`), but that is a second, droppable path — meaning **`workspace_modes` and `workspace_chrome` do not need porting**.
- **The browser does not render on Windows.** `render_handler.rs` gates `RenderState.current_frame: Option<CVPixelBuffer>` behind `#[cfg(target_os = "macos")]`. Upstream GPUI matches: `crates/gpui/src/elements/surface.rs:13` defines `SurfaceSource` with only a macOS `CVPixelBuffer` variant, and `pub fn surface()` (`:33`) is itself macOS-gated. Its only in-tree consumer is `crates/livekit_client/src/remote_video_track_view.rs:81`.
- CEF itself *does* build and run on Windows in Glass (`script/stage-windows-cef-runtime.ps1` stages `libcef.dll`, `*.pak`, `*.bin`, `*.dat`, `locales/`). **Only frame capture and compositing are missing.**
- **The browser's chrome is built from macOS AppKit widgets.** `native_icon_button`, `native_image_view`, `native_tracking_view`, `show_native_popup_menu`, `NativeMenuItem`, `NativeSearchFieldTarget`, `NativeImageScaling`, `NativeImageSymbolWeight` are all confirmed **absent from vanilla upstream GPUI**, used in **56 places across 6 files** (`browser_view/tab_strip.rs` 27, `bookmarks.rs` 10, `browser_view/content.rs` 9, `toolbar.rs` 6, `omnibox.rs` 2, `browser_view/navigation.rs` 2), and **not** cfg-gated. These must be rewritten with Zed's cross-platform `ui` components — expect this to be the bulk of the browser port's effort.
- Minor: `Corner` is also absent upstream and may simply have been renamed; resolve during the port.

### 2. Unified AI terminal (Warp parity)

**Warp is a terminal emulator.** Verified against Warp's own engineering write-up: it implements the VT100 spec and parses ANSI escape sequences over a PTY like any other terminal. "Native" in its positioning is an app-architecture claim (Rust, custom GPU-rendered UI framework, 400+ fps) versus Electron terminals — *not* a claim about bypassing emulation.

What actually gives Warp structured context is two things:

1. **A client-side input editor** — Warp does not let the shell's readline handle input. It built a full text editor as the prompt and reimplemented shell shortcuts (history, `ctrl-r`, tab completion) itself, sending only the completed command to the shell.
2. **Shell hooks emitting DCS** — `precmd`/`preexec` in zsh/fish (`bash-preexec` for bash) emit a Device Control String with encoded JSON metadata, which is how output is cut into blocks with exit codes and timing.

**Zed is unusually well-positioned**, and these should be reused rather than rebuilt:

- `crates/agent/src/tools/terminal_tool.rs` — the agent already runs shell commands, gated by `tools/tool_permissions.rs`.
- `crates/agent_ui/src/terminal_codegen.rs` — **inline AI already exists inside Zed's terminal.**
- `crates/agent_ui/src/message_editor.rs` — already an `editor`-based input, exactly what we want as the prompt.
- `crates/agent_ui/src/agent_panel.rs` + `conversation_view/` — the chat timeline to merge blocks into.

Existing terminal: `crates/terminal` (7.6k lines: `terminal.rs` 5934, `alacritty.rs` 1205, `pty_info.rs` 292) and `crates/terminal_view` (11.3k), wrapping a pinned `zed-industries/alacritty` fork. It has **no concept of a command** — no boundaries, no exit codes, no captured output.

**Design — one block timeline, three block kinds:**

| Block kind | Backing | Notes |
|---|---|---|
| Shell command | PTY + VTE, bounded by OSC 133 marks | argv, cwd, exit code, timing, output as structured data |
| Agent turn | Zed's agent + `terminal_tool` | tool calls and output render as nested blocks |
| App command | Handled in-process by Clay, never sent to the shell | project/workspace navigation, settings, session control |

**Input routing — the AI agent is the default; `!` is the explicit shell escape.**

- Bare input → the agent. Natural language is the primary mode.
- `!`-prefixed input → run verbatim as a shell command, e.g. `!cd C:\Users\Louis\Desktop\Code\Clay`.
- **Failure is not a dead end.** A failed `!` command's argv, cwd, exit code and stderr are fed back to the agent as context and discussed with the user; the agent can then issue corrected commands through its existing terminal access.

This was Louis's correction to an earlier design of mine that used shell-first routing with a natural-language fallback and a heuristic between them. His model is better because there is no heuristic, therefore no silent mis-route: the user always knows which mode they are in from the presence of `!`. It also dissolves the `cd` overloading problem entirely — no special-casing needed.

Watch item: making the agent the default puts its latency on the critical path for ordinary interaction. A bare `git status` must not feel slow. Consider a hint when input exactly matches a known command but lacks `!`.

### 3. From Lathe

Reference checkout: `C:\Users\Louis\Desktop\Code\Clay\_refs\lathe` ([`paterschris/lathe`](https://github.com/paterschris/lathe)), HEAD `4ab1086` "Release v1.0.0".

**Take:**

- **`ai_accounts`** (~1.4k lines) — multi-account AI agent sign-in. Per-agent account index, default-account resolution, on-disk persistence under `accounts_root()`, per-account state, `AiAccountsSettings::resolve_account`. Nothing equivalent upstream.
- **Multi-account collab switcher** — lives in Lathe's `client`/`title_bar` edits, *not* in the `ai_accounts` crate. Includes the copy-collab-link account picker.
- **`pr_ui`** (~6.7k lines) — in-editor PR review for GitHub and Bitbucket Cloud. PR panel (`pull_request_panel.rs`), PR detail/diff view (`pull_request_view.rs`), `submit_review()` with approve / request-changes verdicts (`:180`), `merge()` supporting merge-commit / squash / rebase (`:402-428`), reviewer picker, create-PR modal, `connect_modal.rs` for auth.
- **Workspace groups** — save the open set of projects as a named group, reopen later, optionally bound to an account, portable `.lathe-workspace` file. Added to scope because it is the natural backing store for project navigation in the AI terminal; without it there is no list of projects to navigate.

  Surveyed and **low-risk**: `crates/recent_projects/src/workspace_groups.rs` is 1,011 self-contained lines exposing `list_groups()`, `binding_for()`, `init()` and three modals (Save / Open / Rename). It persists through Zed's existing `KeyValueStore` rather than a schema migration, and touches the `workspace` crate in only ~10 places total (`persistence.rs` 6, `workspace.rs` 3, `persistence/model.rs` 1). This is the cheapest item in the Lathe scope and a good first port.

**Cut:** `mobile_dev` (6.6k, Expo/React Native), `aws_dev` (1.0k).

**Deferred:** theme customizer (135+ colors; cheap and self-contained, just not a priority), and `git_graph` (6.8k) — because upstream already ships `git_ui/src/git_graph.rs` and `git_ui/src/conflict_view.rs`, so Lathe's git work overlaps upstream and must be re-measured before it is worth porting.

**Verified: upstream Zed cannot review, merge or manage PRs.** Its support is link-out only — `git_panel.rs:4593` `create_pull_request()` merely builds a URL via `build_create_pull_request_url` and opens a browser; `blame_ui.rs:376` and `commit_tooltip.rs:325` link to the PR that introduced a commit. There is no PR list, no diff review, no commenting, no merging. Hence taking `pr_ui`.

**Measurement caveat:** Lathe last synced upstream **2026-07-14**, ~7 weeks behind our base. Diffs taken against current Zed HEAD therefore include upstream drift and **overstate** Lathe's real change set — most notably a 73-file / ~3.5k-line `gpui` diff. Re-measure against Zed at Lathe's sync point before budgeting any Lathe port.

### 4. GPUI policy: keep it in-tree

Upstream keeps GPUI in-tree, split into `gpui` / `gpui_platform` / `gpui_macos` / `gpui_windows` / `gpui_linux` / `gpui_web`.

- **Reject Glass's approach.** Glass pins an *external* fork (`Cargo.toml:332-340` points `gpui`, `gpui_macros`, `gpui_platform`, `gpui_tokio`, `gpui_util`, `http_client` and `http_client_tls` at `github.com/Glass-HQ/gpui` @ `3790fca`). What it adds is the macOS AppKit widget layer — worthless on Windows. It also puts their release cadence in front of our builds, drags `http_client` out of our tree, and would make Lathe's in-tree GPUI edits unmergeable.
- **Adopt Lathe's approach:** patch in-tree GPUI. It is what upstream expects and keeps `git merge upstream/main` working.
- **Our additions should be minimal:** a `SurfaceSource::D3D11` variant plus its `gpui_windows` compositing, and nothing else. Replace Glass's `native_*` calls with cross-platform `ui` components rather than porting the AppKit layer.

---

## Phase plan

**Ordering here is superseded by the tracker at the top of this file** — the rebrand, icons and
the AI account switcher were pulled ahead. The gates below still stand as the definition of done
for each body of work.

| Phase | Content | Gate |
|---|---|---|
| **0** | Fork, remotes, baseline build, rebrand, isolation from stock Zed | Stock build runs on Windows; Clay and stock Zed run simultaneously without interference |
| **1** | **Windows surface compositing.** Fill in the existing `draw_surfaces()` stub and add a Windows variant to `PaintSurface` — see "Phase 1 is much smaller than feared" below. Fed by a *throwaway test producer, not CEF*. Keep the macOS `CVPixelBuffer` path working. | An animating externally-owned texture renders inside a GPUI element on Windows, correctly clipped and z-ordered against normal GPUI content |
| **2** | **Port the browser.** Copy `crates/browser` + `toast`, pin `cef`, port the CEF runtime staging script and the helper subprocess binary. Host `BrowserView` as an ordinary pane `Item`. Rewrite the 56 `native_*` call sites with `ui` components. Wire Phase 1's render path into `render_handler.rs`. | Browse a real site in a Clay pane on Windows, with working input, tabs, omnibox, history, downloads. GPUI modals still draw *above* the browser surface. |
| **3** | **Unified AI terminal.** Shell integration + block model; `editor` as client-side input with `!` routing; merge the agent timeline into the block timeline; app-level commands. | Blocks with correct exit codes for ordinary commands; `vim`, `top`, `less`, `ssh` still work; failed `!` commands reach the agent as context |
| **4** | **Lathe port.** `ai_accounts` + collab switcher → `pr_ui` → workspace groups. One crate per commit series. | Each feature works; upstream merge still clean |
| **5** | **Integration.** Connect the three surfaces — terminal blocks offering to open a URL in the browser pane, browser opening localhost from a dev-server block, etc. Deliberately last and deliberately loose; design once the pieces exist. | — |

**Phase 1 is the critical unknown** and gates everything about the browser on Windows. If it fails, the fallback is hosting WebView2 as a native child window — much less graphics code, but it composites *above* GPUI, so no overlays or modals on top of it and z-order fights with Clay's own UI.

---

## Licensing

Zed is AGPL-3.0 (application) / GPL-3.0 (GPUI and foundational crates); Glass is GPL-3.0-or-later; Lathe inherits Zed's terms. Combining them is workable, but Clay is a copyleft application and per-crate `LICENSE-*` declarations must be preserved. `cargo-about` gates CI on third-party license compliance, and adding CEF brings a new dependency tree to clear through `script/licenses/zed-licenses.toml`.

---

## Reference checkouts

| Path | What |
|---|---|
| `C:\Users\Louis\Desktop\Code\Clay` | **Clay itself.** Full clone, `origin` → `LouisHitchcock/Clay`. |
| `C:\Users\Louis\Desktop\Code\Clay\_refs\zed-upstream` | Vanilla Zed @ `dbfeae77`, v1.19.0 — for clean diffs against our changes |
| `C:\Users\Louis\Desktop\Code\Clay\_refs\lathe` | Lathe @ `4ab1086` |
| `C:\Users\Louis\Desktop\Code\OLD\Glass` | Glass (shallow, 1 commit, has uncommitted local edits) |
| `C:\Users\Louis\Desktop\Code\Glass` | Empty and vestigial — predates the Clay naming decision |

---

## Phase 1 is much smaller than feared (verified 2026-09-02)

The project's biggest risk was whether GPUI's Windows renderer could composite an externally-owned texture at all. **It already can — the scaffolding is entirely in place and cross-platform. Only the leaf is missing.**

What already exists, all *not* macOS-gated:

- `Primitive::Surface(PaintSurface)` and `PrimitiveBatch::Surfaces(Range<usize>)` — `crates/gpui/src/scene.rs:222,477`. The surface primitive is a first-class part of the cross-platform scene model.
- The scene collects `scene.surfaces`, and batching/ordering already handles them.
- **`crates/gpui_windows/src/directx_renderer.rs:387` already dispatches** `PrimitiveBatch::Surfaces(range) => self.draw_surfaces(&scene.surfaces[range])`.
- `PaintSurface` already carries `order: DrawOrder`, `bounds: Bounds<ScaledPixels>` and `content_mask: ContentMask<ScaledPixels>` (`scene.rs`), so **draw ordering and clipping are already solved** by the existing machinery.

What is missing is exactly two things:

1. **`PaintSurface.image_buffer` is `#[cfg(target_os = "macos")]`** (`crates/gpui/src/scene.rs`), typed `core_video::pixel_buffer::CVPixelBuffer`. Needs a Windows variant carrying a D3D11 shared-texture handle. Same change shape as the `SurfaceSource` enum in `crates/gpui/src/elements/surface.rs:13`.
2. **`draw_surfaces()` on Windows is an empty stub** — `crates/gpui_windows/src/directx_renderer.rs:810-815` early-returns on empty and then does nothing at all. Needs: open the shared texture by handle (`OpenSharedResource`), create a shader resource view, draw a textured quad honouring `bounds` and `content_mask`.

**Consequence:** Phase 1 drops from "design and thread a new primitive type through the whole renderer" to "add a platform variant to one struct and implement one stub function". The parts I expected to be hard — z-ordering against other GPUI content, clipping, integration with the batching pass — are already done upstream, and those were exactly what the Phase 1 gate was meant to prove.

The WebView2 fallback is correspondingly less likely to be needed. Keep the throwaway-test-producer approach anyway: proving the texture path without CEF in the mix is still the fastest way to isolate failures.

---

## Full isolation plan (decision: entirely separate — no shared state)

Surveyed 2026-09-02. Two pieces of good leverage keep this tractable:

**Leverage 1 — every data directory derives from `data_dir()`.** `extensions_dir()` (`paths.rs:348`), `db_dir()` (`:260`), `themes_dir()` (`:372`), prompts (`:392`), prompt overrides (`:418`), embeddings (`:433`), logs (`:234`), hang traces (`:224`) and remote server state (`:242`) all join onto `data_dir()`. So changing `APP_NAME` (`paths.rs:18`) isolates **all** user data in one edit. No per-directory work needed.

**Leverage 2 — every task-exported variable derives from one constant.** `crates/task/src/task.rs:254` defines `pub const ZED_VARIABLE_NAME_PREFIX: &str = "ZED_";`. Changing it to `"CLAY_"` renames `ZED_WORKTREE_ROOT`, `ZED_FILE`, `ZED_DIRNAME` and the rest of the task-variable surface in a single edit.

### Environment variables

There are ~25 distinct `ZED_*` variables. They fall into three groups that need different treatment:

| Group | Examples | Treatment |
|---|---|---|
| **Read from the user's environment** — real contamination risk, since Louis may have these set for stock Zed | `ZED_STATELESS`, `ZED_DEV`, `ZED_AUTO_UPDATE`, `ZED_RELEASE_CHANNEL`, `ZED_ALWAYS_ACTIVE`, `ZED_DISABLE_STAFF`, `ZED_UPDATE_EXPLANATION` | **Rename to `CLAY_*`.** Highest priority — these are the ones that actually cause cross-talk. |
| **Exported by Clay into terminals and tasks** | `ZED_WORKTREE_ROOT`, `ZED_FILE`, `ZED_DIRNAME`, `ZED_AGENT_ID` | **Rename via the single prefix constant.** Consistent with moving to `.clay/tasks.json`. |
| **Build-time only** | `ZED_COMMIT_SHA`, `ZED_PKG_VERSION`, `ZED_BUILD_ID` | Rename for consistency; no contamination risk, so lowest priority. Note the build script already emits `ZED_COMMIT_SHA`. |

### Ordered work list for the isolation commit

1. `paths.rs:18` — `APP_NAME` `"Zed"` → `"Clay"`. Isolates all user data, config, cache, state and logs.
2. `paths.rs:488,499,506,529` — project dir `.zed` → `.clay` (`settings.json`, `tasks.json`, `debug.json`). **No `.zed/` fallback**, per the decision above.
3. `paths.rs:71,79` — `.zed_server` / `.zed_wsl_server` → `.clay_server` / `.clay_wsl_server`.
4. `paths.rs:245,251` — log filenames follow `APP_NAME`; verify they land as `Clay.log`.
5. `release_channel/src/lib.rs:45-51,228-233` — `app_identifier()` (`Zed-Editor-*` → `Clay-*`) and `app_id()` (`dev.zed.Zed*` → Clay equivalents). **This also isolates the Windows single-instance mutex and named pipe for free**, since `windows_only_instance.rs:34,67` derive their names from `app_identifier()`.
6. `task/src/task.rs:254` — `ZED_VARIABLE_NAME_PREFIX` `"ZED_"` → `"CLAY_"`.
7. Runtime `ZED_*` env vars → `CLAY_*` (group 1 above).
8. URL scheme — `zed://` / `zed-cli://` → `clay://` (`zed/src/main.rs:1809-1810`, `zed/src/zed.rs:1282-1292`, `cli/src/main.rs:34`). Must not steal `zed://` from stock Zed.
9. **Auto-update** (`auto_update/src/auto_update.rs:354-356`) — repoint at Clay's releases or disable. **Dangerous if skipped:** as written it would install stock Zed over Clay.
10. CLI binary name `zed` → `clay` (`crates/cli`).
11. Rename the repo's own `.zed/` directory to `.clay/` so Clay dogfoods its own config.

### Verification for the isolation gate

- Launch Clay and stock Zed **simultaneously**; neither steals the other's window (proves #5) and each keeps its own settings (proves #1).
- Confirm `%LOCALAPPDATA%\Clay` and `%APPDATA%\Clay` are created, and that `%LOCALAPPDATA%\Zed` / `%APPDATA%\Zed` are left untouched — check mtimes before and after.
- Confirm no `ZED_*` variable set in the environment changes Clay's behaviour.
- Confirm auto-update does not point at `zed-industries/zed`.

---

## Build performance: use debug builds for iteration

`Cargo.toml`'s release profile sets `codegen-units = 1` with `lto = "thin"`. That disables parallelism *within* a crate, so the enormous `crates/zed` main crate is compiled by a **single rustc thread** — measured at ~1.8 CPU-hours for that one crate on this machine (rustc peaked around 12.9 GB resident).

That is upstream's correct choice for shipping builds and entirely wrong for development.

**Practice:**

- **Iterate with `cargo build -p zed`** (dev profile: `codegen-units = 16`, `incremental = true`). Essential for Phase 1, which involves repeatedly editing `draw_surfaces()` and rebuilding.
- **Reserve `--release` for packaging and performance checks**, not for verifying that something compiles.
- Cargo takes a lock on the shared target directory, so a debug and a release build **cannot run in parallel**. Only one build at a time.

---

## Phase 0 COMPLETE (2026-09-02)

Committed as `ef8b356` on branch **`clay/rebrand-and-isolate`** (87 files, +614/-293). Not yet merged to `main` or pushed.

### Gates passed

**1. Toolchain gate.** `cargo build --release -p zed` on the unmodified tree: exit 0, 19m19s for the final crate, 422 MB binary.

**2. Compile gate.** `cargo build -p zed` (debug) with all isolation changes: exit 0, ~12 min, produced `target/debug/clay.exe` (509 MB). This is what validated the 152-literal env-var rename.

**3. Isolation gate — passed.** Evidence:

- Clay and stock Zed ran **simultaneously, each with its own window**: `clay` pid 39448 titled "Clay" alongside `Zed` pid 17004. The single-instance mutex is genuinely isolated.
- `%LOCALAPPDATA%\Zed` and `%APPDATA%\Zed` mtimes **byte-identical** before and after launching Clay.
- Clay created its own `%LOCALAPPDATA%\Clay` (`db`, `debug_adapters`, `extensions`, `external_agents`, `hang_traces`, `languages`, `logs`, `prompts`, `threads`) and `%APPDATA%\Clay` (`themes`, `settings.json`, `AGENTS.md`).

### Things learned that cost time

- **`default-run`.** Renaming `[[bin]] name` alone breaks the manifest — `default-run = "zed"` must change too. Failed in 30 seconds, cheaply.
- **`cli/src/main.rs:1279`** locates the app binary by name (`["../Zed.exe", ...]`). This would **not** have failed to compile — only at runtime. Worth grepping for executable-name lookups before any future rename.
- **Background-launched GUI processes get reaped** when the harness task completes, which briefly looked like a mutex collision. Launch detached via `cmd /c start ""` for manual testing.

### `.rules` requires a README marker

`CLAUDE.md` points at `.rules`, which carries a **HARD RULE**: any modification to source files requires `> [!IMPORTANT]` / `> Remove this line to confirm you've reviewed this PR before submitting.` as the first two lines of `README.md`. It is applied. The rule forbids removing those lines — that is explicitly a manual step for the human author.

This is upstream Zed's mechanism for gating PRs into *their* repo. Clay will never submit PRs upstream, so it arguably does not apply here — but it is inherited via `.rules` and is being followed until Louis decides otherwise. **Worth deciding whether to strip this rule from `.rules`**, since otherwise every Clay commit carries a marker aimed at Zed's reviewers.

### Next: Phase 1 — Windows surface compositing

1. Add a Windows variant to `PaintSurface` (`crates/gpui/src/scene.rs`), which is currently `#[cfg(target_os = "macos")] image_buffer: CVPixelBuffer`, carrying a D3D11 shared-texture handle.
2. Implement `draw_surfaces()` in `crates/gpui_windows/src/directx_renderer.rs:810` — currently an empty stub. Needs `OpenSharedResource`, a shader resource view, and a textured quad honouring `bounds` and `content_mask`.
3. Drive it from a **throwaway test producer, not CEF**, so failures are isolated.
4. Iterate with **debug builds** — now incremental, so rebuilds should be fast.

---

## Phase 1 COMPLETE - gate passed 2026-09-02

**The gate passed.** An animating texture, produced on a separate D3D11 device and shared
into the process by NT handle, renders inside a GPUI element - correctly clipped by
`content_mask` and correctly z-ordered beneath overlapping GPUI content. Verified by
screenshot and confirmed on screen by Louis.

Evidence:

- `cargo check -p gpui` and `cargo check -p gpui_windows` both clean, zero warnings. Every
  substitution type-checked first time, including the `HANDLE` cast, the SRV union
  initialiser, and the borrow of `self.pipelines` after the `&mut self` cache lookup.
- Both new shader entry points compile under `fxc.exe` (`vs_4_1` / `ps_4_1`) - the same
  compiler and targets `build.rs` uses for release.
- `cargo build -p zed` clean; Clay launches and renders normally with no visual regression.
  Its log confirms the DirectX path: *"Using GPU: NVIDIA GeForce RTX 4060"*, *"Created device
  with Direct3D 11.1 feature level"*, *"Rendered first frame"*.
- The `surface` example runs. A pixel sampled inside the gradient walks
  `28,163,92 -> 28,183,72 -> 28,203,52` over 1.4s, proving the shared texture is re-read
  every frame rather than frozen on the first.

**This retires the project's largest risk.** The WebView2 fallback is no longer needed: CEF's
`on_accelerated_paint` hands over exactly the kind of shared D3D11 handle this path now
consumes.

### What was written

**`crates/gpui/src/scene.rs`** - new `SharedTexture { handle: isize, size: Size<DevicePixels> }`,
plus a `#[cfg(target_os = "windows")] pub texture: SharedTexture` field on `PaintSurface`.
The handle is an `isize`, not a `windows::Win32::Foundation::HANDLE`, because that type wraps
a raw pointer and is neither `Send` nor `Sync` - using it would have made `Scene` non-`Send`.
The handle is borrowed, not owned: the producer keeps the texture alive and closes the handle.

**`crates/gpui/src/window.rs`** - a `#[cfg(target_os = "windows")] paint_surface(bounds, texture)`
mirroring the macOS one.

**`crates/gpui/src/elements/surface.rs`** - `SurfaceSource::SharedTexture` variant, a `From`
impl, `surface()` now gated on `any(macos, windows)` rather than macos alone, and a paint arm
that fits the texture to the bounds via `object_fit`.

**`crates/gpui_windows/src/shaders.hlsl`** - a new `Surface` struct plus `surface_vertex` /
`surface_fragment`. Modelled on `path_sprite` but, unlike it, applies the content mask via
`distance_from_clip_rect` into `SV_ClipDistance`, following the `quad` convention. The unit
vertex doubles as the texture coordinate, so the texture stretches over the quad and fitting
is the element's job.

**`crates/gpui_windows/src/directx_renderer.rs`** - 14 substitutions:

- `SurfaceSprite { bounds, content_mask }`, the GPU-facing half of `PaintSurface`. Needed
  because a texture handle cannot live in a structured buffer, so geometry and texture are
  uploaded separately. Same pattern as the existing `PathSprite`.
- `surface_pipeline: PipelineState<SurfaceSprite>` with the standard blend state.
- `shared_textures: HashMap<isize, ID3D11ShaderResourceView>` on the renderer, cleared in
  `handle_device_lost_impl`. Opening a shared resource per frame is not viable for a browser
  repainting continuously, and both the views and their textures die with the device.
- `upload_scene_buffers` now uploads surface geometry.
- Dispatch changed from `draw_surfaces(&scene.surfaces[range])` to `draw_surfaces(scene, range)`,
  because the absolute index is needed for `batch_start_index`, not just a slice.
- `draw_surfaces` splits a batch into maximal runs sharing one handle - a draw call can bind
  only one texture - so the common single-surface case stays a single instanced draw.
- `shared_texture_view` opens via `ID3D11Device1::OpenSharedResource1`, since CEF hands over
  an NT handle from `CreateSharedHandle` and the legacy `OpenSharedResource` will not accept it.
- `ShaderModule::Surface`, its `as_str` arm, and its release-bytes arm.

**`crates/gpui_windows/build.rs`** - `"surface"` added to the shader module list, so release
builds get precompiled bytes. Debug builds compile HLSL at runtime from `shaders.hlsl`, which
is why iteration should stay on debug.

### Next steps, in order

1. `cargo check -p gpui_windows` - first compile of any of the renderer work. Expect errors.
2. `cargo build -p zed`, then confirm Clay still launches and renders normally with no
   surfaces in the scene (a pure regression check - nothing should look different yet).
3. Write the **throwaway test producer**: a second D3D11 device creating a texture with
   `D3D11_RESOURCE_MISC_SHARED_NTHANDLE`, `CreateSharedHandle`, and an animated pattern.
   A separate device is the point - it proves genuine cross-device sharing, which is what
   CEF needs. Drive it from a scratch action or a gpui example rendering `surface(...)`.
4. Gate: the animating texture appears inside a GPUI element, correctly clipped by
   `content_mask` and correctly z-ordered against normal GPUI content.

### Open questions deliberately deferred

- **Synchronisation.** The test producer will write while the renderer samples, with no
  fence or keyed mutex, so tearing is possible. Fine for proving the path; CEF integration
  will need `D3D11_RESOURCE_MISC_SHARED_KEYEDMUTEX` or a fence.
- **Alpha.** The surface pipeline uses the standard alpha blend state. If CEF delivers a
  texture with zero alpha in places, content will be invisible; may need an opaque blend
  state or to force alpha to 1 in the fragment shader.
- **Colour space / format.** The SRV takes its format from the shared texture's own desc,
  so BGRA vs RGBA should follow the producer, but this is untested.

### Loose ends found while verifying

- **`Error: could not find zed-cli from any of: bin/zed.exe, ./cli.exe`** on Clay startup -
  stale names left by the binary rename. Non-fatal (Clay starts fine), but should be pointed
  at the Clay CLI names.
- The startup log still reads **"starting zed version 1.19.0+dev"**. Cosmetic branding.
- **A one-character corruption slipped into the rebrand commit**: a stray `s` prefixed onto a
  doc comment in `crates/zed_env_vars/src/zed_env_vars.rs`, which broke the build with
  `expected one of ! or ::, found doc comment`. Almost certainly collateral from one of the
  bulk `sed` passes. Fixed in a follow-up commit.

  The lesson worth keeping: **rebuild immediately before committing, not just at some point
  earlier in the session.** The debug build that would have caught this ran before the last
  few edits, and `git add -A` then committed a tree that had never been compiled. A full
  diff audit afterwards (`git diff upstream..HEAD` filtered for added lines mentioning
  neither `clay` nor `Clay`) found exactly this one line and nothing else, and is a cheap
  check worth repeating after any bulk rename.

### Remaining Phase 1 follow-ups (not blockers)

- **Synchronisation.** Neither the example producer nor the renderer uses a fence or keyed
  mutex, so a partially written frame can be sampled. Not visible in practice here, but CEF
  integration should use `D3D11_RESOURCE_MISC_SHARED_KEYEDMUTEX` or a fence.
- **Alpha.** The surface pipeline uses the standard alpha blend state and the example writes
  alpha 255 throughout, so this is untested against a texture with real transparency.
- **Cache eviction.** `shared_textures` only ever grows and is cleared solely on device loss.
  Fine for a handful of long-lived browser surfaces; would leak if handles churned.
- **`crates/gpui/examples/surface.rs` is throwaway** and should be deleted once the browser is
  the real producer. It is the reason `gpui` has a Windows-only `windows` dev-dependency.

---

## Phase 2 COMPLETE - browser port

### Done

- `crates/browser` (11,852 lines) and `crates/toast` copied from Glass.
- Workspace wired: both added to `members` and `workspace.dependencies`, kept alphabetical.
- `io-surface = "0.16"` added to workspace dependencies. It was the **only** dependency the
  browser needed that Clay lacked - everything else (`db`, `editor`, `fuzzy`, `menu`, `paths`,
  `ui`, `theme`, `settings`, `util`, `workspace`, `zed_actions`, `parking_lot`, `image`,
  `smallvec`, `url`) was already present. Kept rather than dropped so the macOS render path
  stays buildable.
- **`workspace_chrome` and `workspace_modes` dependencies dropped**, confirming the earlier
  survey: neither needs porting.
- Helper binary renamed `glass_helper` -> `clay_helper` (CEF requires a subprocess executable).

### CEF builds on Windows

`cargo check -p browser` resolved `cef v145.6.1+145.0.28`, `cef-dll-sys` and `download-cef`
from `tauri-apps/cef-rs` and started compiling them with no errors. `cef-dll-sys`'s build
script is what downloads the Chromium distribution. This is the Phase 2 viability gate.

### Exact widget inventory (replaces the vague "56 native_* sites")

Glass's browser chrome is built from macOS AppKit widgets in its GPUI fork. The real shape:

| Glass API | Uses | Zed equivalent | Difficulty |
|---|---|---|---|
| `native_icon_button(id, sf_symbol)` | **8 distinct icons only** | `ui::IconButton` + `IconName` | Easy - full mapping below |
| `native_tracking_view(id)` | `.on_mouse_enter` / `.on_mouse_exit` only | `div().id().on_hover(bool)` | Easy - derive enter/exit from the bool |
| `native_image_view(id)` | dual purpose: `.sf_symbol{,_config}()` **or** `.image_uri()` for favicons, plus `.scaling` `.size` `.w` `.h` `.rounded{,_sm}` `.colors` `.flex_shrink_0` | `ui::Icon` for symbols, `gpui::img()` for favicons | Moderate - two paths behind one builder |
| `show_native_popup_menu(items, pos, window, cx, cb)` + `NativeMenuItem` | 5 + 12 | `ui::ContextMenu` | **Hardest** - native menu is index-callback based; ContextMenu is entry+handler based, so genuine redesign |

Icon mapping, all confirmed to exist in `crates/icons/src/icons.rs`:

| SF Symbol | `IconName` |
|---|---|
| `chevron.left` / `.right` / `.up` / `.down` | `ChevronLeft` / `ChevronRight` / `ChevronUp` / `ChevronDown` |
| `arrow.clockwise` | `RotateCw` |
| `arrow.down.circle` | `Download` |
| `xmark` | `Close` |
| `xmark.circle` | `Stop` (semantically better than `XCircle` for stop-loading) |
| `globe` (placeholder) | `Public` - **note Zed has no `Globe`** |

`ui::IconButton` gets `.disabled()` from `Disableable`, `.on_click()` from `Clickable` and
`.tooltip()` from `ButtonCommon`. Glass passes `.tooltip("Go Back")` as a plain string while
Zed's takes a closure, so an adapter must wrap it in `Tooltip::text(...)`.

### Remaining Phase 2 work, in order

1. **Widget adaptation layer** for the four families above. A thin module with matching
   builder surfaces keeps the 56 call sites and future Glass syncs cheap; the call sites get
   renamed off `native_*` so nothing pretends to be AppKit on Windows.
2. **Replace the mode registration** in `browser.rs` (lines 52, 58, 129, 255-292) and the
   `set_titlebar_center_view` calls in `browser_view.rs` (1025, 1359) with a plain action that
   opens `BrowserView` as a pane item. `BrowserView` already implements `workspace::Item`
   (`browser_view.rs:1291`) and `BrowserToolbarStyle::Pane` already exists, so the pieces are
   there.
3. **`tab_strip.rs`** - replace `workspace_chrome::SidebarNavigationList` (lines 12, 160, 191)
   with `ui` components. 27 of the widget sites are in this file.
4. **Wire the Phase 1 render path** into `render_handler.rs`, replacing the macOS-only
   `current_frame` with a shared D3D11 handle on Windows.
5. **Port `script/stage-windows-cef-runtime.ps1`** so `libcef.dll` and friends land beside the
   binary.
6. Build, then browse a real site in a Clay pane.

### CEF builds on Windows - gate passed

`libcef.dll` is 257 MB on disk at `target/debug/build/cef-dll-sys-*/out/cef_windows_x86_64/`,
833 MB total. `cef v145.6.1+145.0.28` resolved from `tauri-apps/cef-rs` and compiled with no
errors. **The browser's Chromium dependency is viable on Windows.**

### `toast` needed three upstream-drift fixes

Glass forked from an older Zed, so its `Component` impl in `status_toast.rs` was stale:
`preview` returned `Option<AnyElement>` where the trait now wants `AnyElement`, the trait
gained a required `description`, and `IconName::GitBranchAlt` was renamed `GitBranch`. Fixed.

### Icon buttons ported - no shim needed

All 8 `native_icon_button` sites became plain `ui::IconButton::new(id, IconName::X)`, and the 8
`.tooltip("string")` calls became `.tooltip(Tooltip::text("string"))` since Zed's
`ButtonCommon::tooltip` takes a closure. Idiomatic, no compatibility layer.

### Full remaining error inventory: 50 errors, categorised

`cargo check -p browser` now gets all the way into the browser's own code. 50 errors, mostly in
`tab_strip.rs` (19), `browser_view.rs` (9) and `browser.rs` (8).

| # | Category | Errors | Fix |
|---|---|---|---|
| A | `theme.component_radius()` missing | 11 | Glass added it to its theme fork. Replace with Clay's radius or a constant. Mechanical. |
| B | `workspace_modes` references | 7 | Delete - the mode system is not being ported. |
| C | `IconName::Globe` missing | 4 | Use `IconName::Public`. Mechanical. |
| D | **Glass's `workspace` fork APIs** | 7 | See below - the only category needing a decision. |
| E | GPUI native widgets | ~6 imports covering ~40 sites | `native_image_view`, `native_tracking_view`, `NativeMenuItem`, `show_native_popup_menu`, `NativeImageScaling`, `NativeImageSymbolWeight`, `NativeSearchFieldTarget`, `Corner`, `focus_native_search_field`. Rework onto `ui::Icon` / `img()` / `div().on_hover()` / `ui::ContextMenu`. |
| F | Misc | 5 | `ButtonSize` vs `Pixels` (3, from the icon-button port), `Keystroke::native_key_code` (2 - Glass added this field for CEF key translation; needs a Windows equivalent), mutability mismatches (2). |

**Category D is the decision point.** Glass's browser calls seven APIs that exist only in
Glass's reworked `workspace` crate:

- `register_browser_mode_url_opener`, `register_embedded_browser_item_factory`
- `WorkspaceItemKind` (both at root and in `item`), and `ItemHandle::workspace_item_kind()`
- `WorkspaceTabsSidebarKind`
- `Workspace::show_browser_surface()`, `::get_mode_view()`, `::close_tabs_entry()`,
  `::terminal_session_manager()`

This is the concrete bill for the earlier decision to keep `crates/workspace` close to upstream
rather than adopting Glass's rework. Most of these are mode/chrome machinery that should simply
be deleted along with category B. `register_embedded_browser_item_factory` is the one that may
warrant a small, genuinely useful addition to Clay's `workspace` so a browser tab can be
restored from serialized workspace state.

### Progress: 50 -> 32 errors

**Browser tabs will persist without touching `crates/workspace` at all.** Zed already has a
complete item-persistence system - the `SerializableItem` trait (`workspace/src/item.rs:409`)
plus `workspace::register_serializable_item::<I>()` (`workspace.rs:1172`). So Glass's
`register_embedded_browser_item_factory` does not need porting: implement `SerializableItem`
for `BrowserView` and register it. Strictly better than the small workspace addition proposed
earlier.

Mechanical categories cleared:

- **11 `component_radius()` sites** collapsed to the default each already supplied via
  `.unwrap_or(px(N))`, so visuals are unchanged and Clay's theme needs no fork. Two of them
  used a bare `theme.` receiver rather than `cx.theme()` and needed a second pass.
- **4 `IconName::Globe`** -> `IconName::Public`.
- **3 `.size(px(18.))`** -> `.icon_size(IconSize::Small)`, since Glass's native button took a
  pixel size where `ui`'s takes a `ButtonSize`.

### The remaining 32 errors, and the decision they force

| Cluster | Errors | Notes |
|---|---|---|
| `tab_strip.rs` - Glass's browser tab sidebar | 19 | `workspace_chrome::SidebarNavigationList`, `WorkspaceTabsSidebarKind`, `Workspace::activate_tabs_entry`/`close_tabs_entry`, plus most native widget sites |
| `workspace_modes` references | 6 | `browser.rs:52`, `browser_view.rs:39,1025,1026,1359,1360` |
| Glass `workspace` fork APIs | ~5 | `WorkspaceItemKind`, `ItemHandle::workspace_item_kind`, `register_browser_mode_url_opener`, `get_mode_view`, `terminal_session_manager`, `show_browser_surface` |
| GPUI native widgets outside tab_strip | ~6 | `native_image_view` (favicons), `show_native_popup_menu` + `NativeMenuItem`, `NativeSearchFieldTarget`, `Corner`, `focus_native_search_field` |
| `Keystroke::native_key_code` | 2 | Glass added this field for CEF key translation; needs a Windows equivalent |

**`crates/browser/src/browser_view/tab_strip.rs` is 1,022 lines implementing
`BrowserSidebarPanel`** - Glass's in-view browser tab sidebar. It is used from `browser_view.rs`
in only 4 places (lines 9, 12, 216, 1499, 1505). It accounts for 19 of the 32 remaining errors
and is the sole consumer of `workspace_chrome`.

This is precisely the Glass window chrome the approved plan said to rework rather than copy.
Deleting it means choosing the **one page per Zed pane tab** model: Zed's own pane tabs become
the browser tabs, sitting beside editor tabs. That is what the plan describes, and Glass already
has a `BrowserPaneItem` (`browser_view.rs:1226`) for exactly this. The cost is losing Glass's
in-view tab strip and its pinning behaviour.

### Per-project tabs: upstream Zed already has this

Louis wants multiple projects open in one Clay window, tabbed between quickly, rather than one
window per project. **`crates/workspace/src/multi_workspace.rs` (2,199 lines) already implements
exactly that**, and it is upstream Zed code, not something Glass added:

- A workspace switcher sidebar, "one row per workspace held by this window"
- Actions: `ToggleWorkspaceSidebar`, `FocusWorkspaceSidebar`, `NextProject`, `PreviousProject`,
  `MoveProjectUp`, `MoveProjectDown`, `MoveProjectToNewWindow`, plus thread equivalents
- `ProjectGroup`, `ProjectGroupState` and `SerializedProjectGroupState`, so the arrangement
  persists across restarts
- Already bound: **`ctrl-alt-j`** toggles it in `assets/keymaps/default-windows.json:672`, and
  it is wired into `crates/zed/src/main.rs` via `get_any_active_multi_workspace`

**The catch, and the one change Clay wants:**

```rust
pub fn multi_workspace_enabled(&self, cx: &App) -> bool {
    !DisableAiSettings::get_global(cx).disable_ai && AgentSettings::get_global(cx).enabled
}
```

The sidebar is a combined **projects + agent-threads** switcher (hence `NextThread` / `NewThread`),
so upstream ties it to the agent being enabled. For Clay the project switcher should work
regardless of AI settings, so decoupling that gate is a small, well-targeted change.

This also dovetails with the unified AI terminal: a switcher that already lists both projects
and agent threads is the natural home for that surface.

*Caveat: an automated `ctrl-alt-j` keystroke test did not visibly open the sidebar in a
screenshot. That may be the AI gate, or simply SendKeys not reaching the window. The code, the
keybinding and the wiring are all confirmed present by inspection; the runtime behaviour is not
yet confirmed.*

### Sidebar decision resolved: delete Glass's, build on upstream's

Glass's `BrowserSidebarPanel` (`tab_strip.rs` regions 3 and 5, ~550 lines) should be **deleted,
not ported**. Upstream's `multi_workspace` sidebar is better integrated, already persists, and
is the thing Louis actually wants. Porting Glass's would mean building a second, worse sidebar
system alongside it.

Revised plan for `tab_strip.rs`:

1. **Port** `show_tab_context_menu` (19-93) onto `ui::ContextMenu`. Note the callback shape
   changes: Glass's native menu is index-based (`|action_index, ...| if action_index == 0`),
   `ContextMenu` is entry-plus-handler, so dispatch must be restructured rather than translated.
2. **Port** `render_tab_favicon` (94-112) - `img(url)` with an `Icon::new(IconName::Public)`
   fallback.
3. **Port** `render_tab_strip` (243-600) - mechanical: 3 tracking views to `div().on_hover()`,
   2 image views to `img`/`Icon`.
4. **Delete** `BrowserSidebarPanel` (113-242) and `render_sidebar` / `ensure_native_sidebar_panel`
   (601-1022), plus their 4 call sites in `browser_view.rs` (lines 9, 12, 216, 1505).
5. Separately, **decouple `multi_workspace_enabled` from the AI settings** so per-project tabs
   work unconditionally.

### Phase 2 progress: 50 -> 9 errors, all architecture done

Every remaining error is the AppKit widget block. **Caveat: 6 of the 9 are unresolved-import
errors that currently mask ~40 downstream call sites**, so the count understates the work left.
Nothing architectural remains.

**Sidebar deleted (552 lines).** `BrowserSidebarPanel` (130 lines) and
`render_sidebar`/`ensure_native_sidebar_panel` (422 lines) removed from `tab_strip.rs`, which
went 1,022 -> 470 lines with braces balanced. Also removed: the `TabBarMode::Sidebar` variant
and its render arm, `toggle_sidebar` / `handle_toggle_sidebar` and their action registration,
two sidebar-notify blocks in `tabs.rs`, and the `workspace_chrome` dependency entirely.
`session.rs` now pins `tab_bar_mode` to `Horizontal` and serialises `sidebar: false`, keeping
the field so existing saved sessions still deserialise.

**Mode system removed.** Deleted the `ModeViewRegistry` factory (49 lines) and the four
navigation-host helpers plus `open_browser_mode_url` (63 lines) from `browser.rs`, and the three
now-dead navigation functions from `browser_view.rs` (`navigation_entries`,
`activate_navigation_entry`, `close_navigation_entry`, 50 lines). Glass pushed the browser
toolbar into the title bar via `set_titlebar_center_view`; both calls are gone, since Clay's
browser toolbar belongs to the pane toolbar.

**Browser now opens as an ordinary tab.** `OpenBrowserPane` used to call Glass's
`workspace.show_browser_surface(...)`; it now calls a new `open_browser_tab()` that creates a
`BrowserView`, wraps it in the existing `BrowserPaneItem`, and calls
`workspace.add_item_to_active_pane(...)`.

**Toolbar reworked for the pane-item model.** Glass had one global browser, so
`attach_browser_toolbar_to_pane` fetched it via `workspace.get_mode_view(ModeId::BROWSER)` and a
workspace-wide attach also walked `terminal_session_manager()` panes. Clay attaches on demand
when a `BrowserPaneItem` lands in a pane, taking the view from the item itself via a new
`BrowserPaneItem::browser_view()` accessor. `WorkspaceItemKind::Browser` tagging was replaced
with `item.downcast::<BrowserPaneItem>()`, matching how Zed identifies item types elsewhere.

### What is left

| Item | Sites | Notes |
|---|---|---|
| `native_image_view` | ~15 | Two forms: `.image_uri(url)` -> `img(url)`, and `.sf_symbol("globe")` -> `Icon::new(IconName::Public)` |
| `native_tracking_view` | ~9 | Needs real rewriting, not a sed: Glass has separate `on_mouse_enter`/`on_mouse_exit` closures, GPUI's `div().on_hover()` reports a single bool, so the two closures must merge into one branch |
| `show_native_popup_menu` + `NativeMenuItem` | 5 + 12 | Hardest: index-callback dispatch -> `ui::ContextMenu`'s entry+handler model |
| `NativeSearchFieldTarget` / `focus_native_search_field` | 3 | Omnibox focus; the crate already depends on `editor` |
| `Keystroke::native_key_code` | 2 | Glass added this field for CEF key translation; needs a Windows virtual-key equivalent |
| `Corner`, `NativeImageScaling`, `NativeImageSymbolWeight` | - | Styling params that fall away with the above |
| **`SerializableItem` for `BrowserView`** | - | Not yet started. Required for Louis's browser-tab persistence, via `workspace::register_serializable_item` |

### Browser crate compiles, and the Windows render path is wired

`cargo check -p browser` is **clean**. The remaining widget work was resolved as follows:

- **`navigation.rs`** - the AppKit `focus_native_search_field` call was removed; the
  `#[cfg(not(target_os = "macos"))]` branch immediately below it already focused the omnibox
  through the toolbar, so that path is now unconditional.
- **`input.rs`** - `Keystroke::native_key_code` was a Glass addition carrying a *macOS* virtual
  keycode. Upstream GPUI has no such field on any platform, so the existing
  `key_name_to_windows_vk(&keystroke.key)` fallback is now the only path, which is what CEF
  wants on Windows anyway. `macos_keycode_to_windows_vk` is kept but marked `dead_code`.
- **`Corner` -> `Anchor`** - upstream renamed the type; variants are identical.
- **Image views** - `.image_uri(url)` became `img(url)`, and `.sf_symbol("globe")` became
  `Icon::new(IconName::Public)`. The 96px placeholder uses `IconSize::Custom(rems(6.0))`,
  since `Icon` has no `custom_size`.
- **Tracking views** - a small `HoverTracker` helper keeps Glass's `on_mouse_enter` /
  `on_mouse_exit` builder shape while dispatching from GPUI's single-boolean `on_hover`. It
  delegates `Styled` to a wrapped div, because the call sites position themselves. This kept
  9 intricate call sites unchanged instead of rewriting each by hand.

**Context menus are stubbed, not ported.** Both bookmark menus and the tab menu are `TODO`s
with their handlers intentionally inert. Glass's AppKit popup dispatches on an item *index*
from one callback, whereas `ui::ContextMenu` takes a handler per entry and needs menu state
plus a dismiss subscription on the owning view. That is a restructure rather than a
translation, and it is not on the path to seeing a web page, so it was deferred.

**Windows render path (the Phase 1 work, now connected to CEF):**

- `RenderState::current_frame` gains a `#[cfg(target_os = "windows")] Option<SharedTexture>`.
- `on_accelerated_paint` reads `info.shared_texture_handle` - cef-rs types this as a raw
  `*mut c_void` on Windows, not a newtype, so it is used directly. The texture belongs to
  CEF's GPU process and is only borrowed; its size comes from `RenderState`, which `view_rect`
  already maintains.
- `FrameReady` was macOS-gated in three places (`events.rs`, `tab.rs` twice, `browser_view.rs`);
  all now cover Windows.
- `content.rs` draws `surface(frame)` on Windows as well as macOS.

**Wired into the app:** `browser.workspace = true` added to `crates/zed`, `browser::init(cx)`
called beside `terminal_view::init(cx)`, and `browser::handle_cef_subprocess()` called early in
`main` - CEF re-executes the binary for helper processes, and a subprocess never returns from
that call.

*Watch item:* the first placement of that call landed between `#[cfg(unix)]` and
`util::prevent_root_execution()`, which would have silently disabled CEF on Windows **and**
stripped the unix gate off root-execution prevention. Caught by reading the result back;
a reminder that inserting before an anchor line can capture a preceding attribute.

**`script/stage-windows-cef-runtime.ps1`** written for Clay. Unlike Glass's, it stages into the
target directory *beside the binary* rather than a `cef_runtime/` subdirectory, because the
loader must find `libcef.dll` at process start and CEF resolves `*.pak` / `icudtl.dat` /
`locales/` relative to the executable. It skips unchanged files, which matters since
`libcef.dll` alone is ~250 MB.

Still outstanding: `SerializableItem` for browser-tab persistence, the context menus, and
renaming the `GLASS_CEF_DEBUG` env var to `CLAY_CEF_DEBUG`.

### Browser is running: CEF initialises on Windows

`cargo build -p zed` is clean with the browser wired in, and Clay launches with CEF live -
a helper subprocess (~100 MB) sits alongside the main process, and `browser::init` logs no
error, which it would if `CefInstance::initialize` had failed.

**Two bugs found getting there, both worth remembering:**

1. **`Copy-Item -LiteralPath` does not expand wildcards.** The staging script's
   `Copy-Item -LiteralPath (Join-Path $localesSource "*")` silently copied *nothing*, so
   `locales/` was created but empty. CEF hard-fails without its locale paks: the process died
   with `STATUS_BREAKPOINT` (`0x80000003`) **before logging was initialised**, which made it
   look like a crash on startup rather than a missing resource. Fixed by using `-Path`.
2. **Two `cef-dll-sys-*` build directories exist** with identical timestamps, and two more that
   are empty. The "newest `libcef.dll`" heuristic can pick either; both happened to be complete,
   but a staging script should prefer a directory that actually contains `locales/`.

Staged beside the binary: `libcef.dll` (245 MB), `chrome_elf.dll`, `d3dcompiler_47.dll`,
`libEGL.dll`, `libGLESv2.dll`, `vk_swiftshader.dll`, the three `.pak` files, `icudtl.dat`,
`v8_context_snapshot.bin` and 220 locale files. Note the distribution has no `snapshot_blob.bin`
(modern CEF only ships `v8_context_snapshot.bin`) and does include `bootstrap.exe`, which is not
currently used.

**`browser::OpenBrowserPane` is bound to `ctrl-alt-n`** in `assets/keymaps/default-windows.json`.
Dev builds read assets from the checkout at runtime (`util::fs_embed!`), so keymap edits apply on
next launch with no rebuild.

**Not yet confirmed: that a browser tab actually renders a page.** Automated keystroke injection
proved unreliable - `SendKeys` went to whatever window held focus rather than Clay - so this
needs a human pressing `ctrl-alt-n`. Everything up to that point is verified.

### How to test

1. Run `target\debug\clay.exe`.
2. Press **`ctrl-alt-n`**, or use the command palette (`ctrl-shift-p`) and pick
   **`browser: open browser pane`**.
3. A browser tab should open beside the editor tabs.

Expected gaps: right-click menus do nothing (stubbed), and browser tabs do not yet survive a
restart (`SerializableItem` not implemented).

### Phase 2 gate PASSED - a real page renders in a Clay pane on Windows (2026-09-03)

Verified on screen: `example.com`, `example.org`, `rust-lang.org`, a Google search and a
logged-in Instagram profile all render as page pixels inside a browser pane, with correct
colours, working omnibox, navigation buttons, tab titles and favicons.

#### The blocker: CEF's shared texture is callback-scoped

The page had been loading correctly for some time - correct titles and favicons came back - but
`ID3D11Device::OpenSharedResource` **and** `OpenSharedResource1` both returned `E_INVALIDARG`
for the handle from `on_accelerated_paint`, and the content area stayed on "Loading...".

CEF's own header documentation settles it, in the bindings at
`sys/src/bindings/x86_64_pc_windows_msvc.rs` (line 11473 for `on_accelerated_paint`, 1147 for
`_cef_accelerated_paint_info_t`):

> on Windows it is a HANDLE to a texture that can be opened with D3D11 **OpenSharedResource1**
> or D3D12 OpenSharedHandle... The underlying implementation uses a **pool** to deliver frames.
> As a result, the handle may differ every frame depending on how many frames are in-progress.
> **The handle's resource cannot be cached and cannot be accessed outside of this callback. It
> should be reopened each time this callback is executed and the contents should be copied to a
> texture owned by the client application.** The contents of |info| will be released back to the
> pool after this callback returns.

And on the handle field: *"The shared texture is instantiated without a keyed mutex."*

So the handle is an NT handle, valid only for the duration of the callback. Storing it and
opening it later on the render thread - which is what the design did - could never work. The
diagnostic that pointed here was the handle changing almost every frame while the size stayed
correct.

**Phase 1 did not catch this** because its test producer created one long-lived NT-handle
texture. It validated the compositing path but never the handle-lifetime assumption. Worth
remembering as a lesson about what a stand-in producer does and does not prove.

#### The fix

`crates/browser/src/frame_bridge.rs` does what CEF asks. A `FrameBridge` opens CEF's texture
inside the callback with `OpenSharedResource1`, `CopyResource`s it into a long-lived texture it
owns (`SHARED_NTHANDLE | SHARED`, `BIND_SHADER_RESOURCE`), and `Flush()`es. GPUI is handed a
handle to *that*, which stays stable while the frame size and format hold, so the renderer can
still cache a shader resource view for it.

- **One device serves every tab.** The bridge device is a process-wide lazy static with the
  immediate context behind its own mutex. A D3D11 device is free-threaded; the immediate context
  is not, and is reachable only through that mutex.
- **The adapter must match GPUI's.** DXGI cannot share a texture across adapters, so the bridge
  walks `EnumAdapters` from zero and keeps the first that yields a D3D11 device - deliberately
  the same walk as `gpui_windows::directx_devices` - and logs the adapter name so a mismatch is
  diagnosable against GPUI's own `Using GPU:` line.
- **No keyed mutex or fence.** CEF creates its textures without one, and Clay runs CEF with
  `external_message_pump = 1`, which puts the callback on the main thread - the same thread that
  draws the scene - so the copy and the draw are already ordered.

`SharedTexture` gained an `id: u64`. Windows recycles handle *values*, so a producer that
replaces its texture on a resize can be handed back a numeric handle the renderer already has a
cached view for, and would then keep drawing the old texture. `directx_renderer.rs` now keys its
cache and its draw-run splitting on the id, and clears the cache past 8 entries so a drag-resize
cannot grow it without bound.

#### Three more bugs found once pixels were on screen

1. **No omnibox.** A pane holds one toolbar item per type, so a single `BrowserToolbar` serves
   every browser tab in the pane - but `set_active_pane_item` only decided visibility, so the
   toolbar stayed bound to whichever browser opened first. That was a new tab page, where the
   omnibox is deliberately hidden. It now rebinds to the active item's browser view and observes
   that view, so switching tabs inside the browser follows too.

   Two supporting faults: `BrowserView` owned a second, never-rendered `TitleBar`-style toolbar
   (the render tree had lost its child, leaving a dangling `#[cfg(not(target_os = "macos"))]`
   attached to the macOS branch), and `FocusOmnibox` was focusing *that*, so ctrl-L did nothing.
   And both "if changed" guards in the toolbar compared entity ids without checking whether a
   subscription existed, which skipped the *initial* bind - leaving the omnibox with no
   `AddressChanged` subscription, so it showed its placeholder rather than the current URL.

2. **A stranded tab on every launch.** `open_browser_tab` called `add_tab` unconditionally, but
   `BrowserView::new` already leaves a tab behind - the restored session, or a new tab page if
   there was nothing to restore. The extra tab was then saved back out, so the count grew by one
   per run. Now conditional on `has_tabs()`.

3. **A restored session rendered blank.** Exposed by fixing (2), since a restored tab is now the
   one in front. `create_browser` succeeded and the load started, but `on_accelerated_paint`
   never fired. **A windowless CEF browser created before the message loop has run never begins
   painting** - and `ensure_browser_created` was creating the browser *before* starting the pump,
   the only path that did. Split into `start_message_pump_if_ready` and
   `ensure_active_tab_browser`, so the pump starts on one pass and the browser is created on the
   next. That also means any restored tab loads the first time it comes to the front.

Also fixed: `cargo test -p browser` had never compiled, because two `#[cfg(test)]` helpers built
a `gpui::Keystroke` with a `native_key_code` field that upstream no longer has. 25 tests pass.

#### Driving the UI for verification

`SendKeys` automation works, but **only** when gated on `GetForegroundWindow()` matching Clay's
window. An ungated attempt typed a search into a browser window that happened to hold focus.
`SwitchToThisWindow` is more reliable than `SetForegroundWindow` for raising the window first.

Note also that Clay's log file is buffered and flushes only on exit, so `log::` output is
invisible at runtime and lost entirely if the process is killed. Use `eprintln!` and capture
stderr; `_refs/run_clay.bat` redirects to `_refs/clay_stderr.txt`.

#### Remaining Phase 2 gaps

- Right-click menus are stubbed (`TODO`s in `tab_strip.rs` and `bookmarks.rs`).
- `SerializableItem` is not implemented, so a browser *pane* does not survive a restart, even
  though the tab list does. Use `workspace::register_serializable_item::<BrowserView>`.
- `ctrl-alt-*` keybindings do not fire on this machine, almost certainly AltGr on a UK layout.
  The `ctrl-k ctrl-b` chord did not fire either; the command palette is the reliable route.
- `ZED_*` variables in `script/` and `.github/` were not renamed, and `GLASS_CEF_DEBUG` should
  become `CLAY_CEF_DEBUG`.
- The renderer's "scene too large" error prefix is misleading - it wraps every batch failure.
