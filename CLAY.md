# Clay — project state

Clay is a fork of [Zed](https://github.com/zed-industries/zed) that combines an inline browser, a unified AI terminal, and selected tooling from other Zed forks.

**This file is the source of truth for "where are we".** Update it after each meaningful unit of work. It exists so that work can resume cleanly across sessions without reconstructing context from a lost conversation.

Full design rationale lives in the plan file: `C:\Users\Louis\.claude\plans\binary-wandering-cray.md`.

---

## Current status

**Phase 0 (fork, build, baseline) — COMPLETE.** See "Phase 0 COMPLETE" below for gate evidence; the table here is the per-item record.

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

All Phase 0 actions are complete. Next up is **Phase 1 — Windows surface compositing**; see the end of this file for the concrete steps.

Outstanding housekeeping: `clay/rebrand-and-isolate` is unpushed and unmerged to `main`.

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
