# Handover: add-picker-v1

You are picking up implementation of OpenSpec change `add-picker-v1` in
`/Users/kahi/Desktop/code/plugins/tmux/pane-mux`. Planning is complete and
validated (`openspec validate add-picker-v1 --strict`). Nothing has been
implemented yet: the repo contains only `openspec/`, `.pi/` and `.gitignore`.

Use the `openspec-apply-change` skill (`.pi/skills/openspec-apply-change/SKILL.md`)
and tick checkboxes in `tasks.md` as you go.

---

## 1. Read these first (in order)

| File | Why |
|---|---|
| `proposal.md` | Scope, and what is explicitly NOT in V1 |
| `specs/tmux-picker/spec.md` | Main behaviour contract, every scenario is a test case |
| `specs/session-switching/spec.md` | `switch-session` command |
| `specs/configuration/spec.md` | Config paths, merge rules, keymap layering |
| `design.md` | Decisions D1-D14: layout, crates, loop, tmux trait, config merge |
| `tasks.md` | Ordered checklist; 2.1 and 2.2 are already done (spikes) |

---

## 2. About the user (important)

- **This is the user's first Rust project.** They know TypeScript/Bun well.
  Prefer simple, idiomatic, readable Rust over clever Rust. Avoid lifetimes
  gymnastics, generics-heavy APIs, macros (except the one preset macro in D9)
  and async. When you introduce a Rust concept, a one-line comment or a short
  note in your summary helps.
- The user models CLIs on their TS CLI `llw`
  (`/Users/kahi/Desktop/code/plugins/ts-packages/llw`): noun/verb commands, one
  wrapper around the external CLI, errors mapped once in `main`, data on stdout,
  diagnostics on stderr.
- The UX and config are modelled on **yazi**
  (`/Users/kahi/Desktop/code/editor_extras/cli_tools/yazi`). Borrow ideas, not
  architecture: no tokio, no Lua, no workspace, no custom terminal backend.
- The user's yazi config is the source of defaults:
  `/Users/kahi/dotfiles/yazi/keymap.toml`, `/Users/kahi/dotfiles/yazi/theme.toml`.

---

## 2a. Council members you can ask

This project runs in the p2p council **`pmux`**. Run `p2p_ls` to confirm who
is connected (names and working directories may change). Ask members with
`p2p_ask` instead of reading their whole codebases yourself. They already know
their code and give short, path-referenced answers.

| Member | Working directory | Ask them about |
|---|---|---|
| `yazi` | `/Users/kahi/Desktop/code/editor_extras/cli_tools/yazi` | How yazi does something you are porting: key parsing (`<C-u>`, sequences), `run` string parsing, prepend/append keymap merge, the which popup state machine, input/confirm/help popups, theme TOML → ratatui `Style`, preset embedding, config dir resolution, layout ratio. |
| `llw` | `/Users/kahi/Desktop/code/plugins/ts-packages/llw` | The user's own TS CLI conventions: command layout, how the external CLI wrapper is structured and tested, error/exit handling, output conventions, lint/format/test scripts. Ask when unsure how the user would like something organised. |

How to ask well:
- Ask one specific question with context, e.g. *"In yazi, how does `Which` decide
  between dispatching and waiting when `g` is both a full chord and a prefix?
  File paths plus a short snippet, under 40 lines."*
- Tell them to stay **read-only**. They must not edit their repos for this project.
- Use one `p2p_ask` call with both members when you need both answers.
- Treat their replies as references to verify, not as specs. For a conflict,
  the order of precedence is: this change's specs/design, then the user, then council answers.
- Questions about **intent or scope** (anything in section 3, or anything the
  specs don't cover) go to **the user**, not the council.

Already asked during planning (don't repeat): a full yazi architecture map
(crates, keymap, theme, config, event loop, popups, layout) and a full llw
structure map (tree, command registration, process wrapper, tooling,
conventions). The conclusions are already in `design.md`, so only ask follow-ups
about specifics.

---

## 3. Decisions locked with the user (do not change without asking)

- `pane-mux picker [--session|--window|--pane]`: all three columns always
  navigable; the flag only decides **what is printed** on `<Enter>`
  (`--session` on a hovered pane prints that pane's session).
- Picker **prints** the target to stdout; it does not attach or switch. There
  is **no** `--action` flag and **no** attach action.
- `pane-mux switch-session`: picker + `switch-client` to whatever level was
  picked; **outside tmux, exit with an error** (no fallback).
- Preview: full-width rounded box, `preview_position` top|bottom (default
  bottom), `preview_ratio` (default 50). Session hover previews active
  window → active pane; window hover previews its active pane.
- Never preview the picker's own pane **unless** running in a popup.
- Cannot delete the current session/window/pane. Creating **panes** is out of
  scope; sessions and windows can be created.
- Keys: `<PageUp>/<PageDown>` page the list; `<C-u>/<C-d>` scroll the
  preview; `K/J` are dropped.
- Icons: session ``, window `󱂬`, pane `` (Nerd Font glyphs; copy them from
  `specs/tmux-picker/spec.md`, they are easy to lose).
- Config dir: `~/.config/pane-mux/` with `config.toml`, `keymap.toml`,
  `theme.toml`; defaults embedded yazi-style (debug: read from disk, release:
  `include_str!`).
- crates.io publishing is **deferred** until after V1; the user will discuss
  the pipeline later. Do not add release tooling.

Proposed by the planner and written into the specs, but never explicitly
approved by the user. Implement them as written, and mention them in your
summary so the user can object:
exit codes 0 / 130 / 2; target format `work` / `work:1` / `work:1.0`; no flag →
focused column's level; `r` = rename; `PANE_MUX_CONFIG_HOME` + XDG lookup;
`deny_unknown_fields`.

---

## 4. Verified facts (no need to re-check)

- Environment: macOS 26, tmux 3.8, rustc/cargo 1.99.1, rustup installed.
  The development shell runs **inside tmux**.
- **Not installed:** `just` (`brew install just`) and `cargo-insta`
  (`cargo install cargo-insta`). Install them, or ask the user, during task 1.5.
- **Popup detection (task 2.1, verified):** inside `display-popup`, `TMUX` is
  set and `TMUX_PANE` is **unset**. `tmux display -p '#{pane_id}'` inside a
  popup returns the opening client's active pane. The rule is in design.md D7.
- **switch-client (task 2.2, from `man tmux`):** `-t` with `:`, `.` or `%`
  switches session + window + pane in one call. No `select-pane` follow-up.

---

## 5. Order of work

```
 [1 Setup] ---> [2 tmux layer] ----+
     |                             +--> [4 Picker core] --> [5 UI + loop] --> [6 Commands] --> [7 Integration]
     +-------> [3 Config] ---------+
                   ^
                   +-- 3.4 needs the Action enum (define it first, see below)
```

### Step 0: before anything else
Run `git status`. The repo has **no commits** yet. Ask the user whether they
want an initial commit of the planning files before code lands.

### Step 1: Group 1, setup (sequential, do it yourself)
Tasks 1.1-1.6. Everything else depends on it. Use edition 2024. Commit
`Cargo.lock` (this is a binary). Keep `rustfmt.toml` to stable-only options.

### Step 2: de-risk the terminal early (recommended deviation)
Before deep work, build a **throwaway vertical slice**: a ratatui frame drawn
on **stderr**, keys read via crossterm `use-dev-tty`, `q` exits, a hardcoded
string printed to stdout. Verify `x=$(cargo run -- picker); echo "$x"` shows
the UI and captures only the string, and the terminal is clean afterwards.
This proves design D3, the riskiest assumption. Fold the code into task 5.5
later.

### Step 3: two parallel lanes
These touch disjoint folders, so they can be given to separate workers:
- **Lane A, `src/tmux/`:** tasks 2.3 → 2.4 → 2.5 → 2.6.
- **Lane B, `src/config/`:** tasks 3.1 → 3.2 → 3.3 → 3.6 → 3.7 → 3.8.

**Shared dependency:** 3.4/3.5 (keymap) parse `run` strings into the `Action`
enum, which lives in `src/picker/action.rs` (design D11). Define that enum
**first**, data only with no behaviour, so lane B can parse into it. Then do
3.4 and 3.5.

### Step 4: Group 4, picker core (after lanes A and B)
State and action logic tested against `FakeTmux`, with no terminal. Order:
4.1 → 4.2 → 4.3 → 4.5 → 4.4 → 4.6 → 4.7 → 4.8. Getting `pick` (4.5) working
early gives an end-to-end path sooner.

### Step 5: Group 5, UI + event loop
5.1 → 5.2 → 5.3 → 5.5/5.6 (from the Step 2 slice) → 5.4 (popups last).
Use `insta` snapshots on `ratatui::backend::TestBackend`.

### Step 6: Group 6, commands, then Group 7, integration
The manual checks in 7.2 need the user's terminal; hand them a checklist
instead of claiming they pass.

---

## 6. Gotchas

- **Never write UI to stdout.** Only the final target line goes there. Restore
  the terminal (raw mode off, leave the alternate screen) before printing,
  **and in a panic hook**.
- **Tests must not touch the user's real tmux server.** The dev shell is
  inside a live tmux session (`$TMUX` is set). Real-tmux integration tests use
  a private socket (`tmux -L pane-mux-test ...`) or an isolated `TMUX_TMPDIR`,
  gated behind an env var, and must clean up their server. Unset `TMUX` and
  `TMUX_PANE` in child test processes where relevant.
- **Do not run the interactive picker from an agent shell** expecting it to
  finish; it blocks on keyboard input. Test logic through `FakeTmux` and
  snapshots, and leave interactive checks to the user.
- Address items internally by tmux **ids** (`$3`, `@5`, `%12`) so the cursor
  survives refreshes (D6). Print **names/indexes** for targets.
- Build the tree from **one** `list-panes -a -F` call with tab-separated
  fields (D5). Window names may contain spaces.
- Don't use `dirs::config_dir()`: on macOS it returns `~/Library/Application Support`.
- `capture-pane -e -p -J` output contains ANSI; convert with `ansi-to-tui`.
- Keymap merge rule (yazi): user `keymap` replaces the layer; `prepend` wins;
  `append` only fills unbound keys; "bound" compares the `on` prefix.
- yazi reference code worth reading (or ask the `yazi` council member about it):
  - `yazi-core/src/which/which.rs`: the multi-key "which" state machine (~60 lines, port it).
  - `yazi-config/src/keymap/{key,chord,section}.rs`: key parsing and layer merge.
  - `yazi-config/src/mixing.rs`: prepend/append dedup.
  - `yazi-macro/src/asset.rs`: the debug-from-disk / release-`include_str!` preset trick.

---

## 7. Definition of done

- Every box in `tasks.md` is checked, each verified as its task describes.
- `just check` passes (fmt --check, clippy -D warnings, tests).
- `cargo install --path .` works, and the user has run the 7.2 manual scenarios.
- `README.md` documents commands, exit codes, a scripting example, a
  `display-popup` binding, and config files/defaults.
- Do **not** archive the change or set up publishing; the user decides both.
