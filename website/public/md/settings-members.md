# Settings members

Manage workspace seats, active members, pending invites, teams, bulk selection actions, role/plan pickers, CSV export, and verified domain auto-join.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add settings-members`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::settings_members::{SettingsMembersScreen, SettingsMembersEvent};

let screen = cx.new(|cx| SettingsMembersScreen::new("settings-members", cx));
cx.subscribe(&screen, |_, event: &SettingsMembersEvent, _| {
    let _ = event;
}).detach();
```

## Interaction

Filter by search or Role, invite new emails with Owner/Admin/Member/Guest roles, toggle @acme.com domain auto-join, and switch Members, Invites, and Teams.

## API

Primary GPUI type: `SettingsMembersScreen` (`kui::components::settings_members::SettingsMembersScreen`).
Event type: `SettingsMembersEvent`.

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_loading(true)` skeleton placeholders, `with_empty(true)` empty states, and responsive desktop-to-mobile adaptation.

## Motion

Interactive transitions, live status indicators, and skeleton sweeps follow KUI motion curves and respect Theme::reduced_motion. Responsive breakpoints adapt columns, toolbars, and side inspectors cleanly from full desktop down to 390px mobile viewports.

Desktop and web use the same GPUI source.
