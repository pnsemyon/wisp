# Web UI

#ui #frontend

`ui/` is Wisp's frontend: a single HTML page (`ui/index.html`), one stylesheet
(`ui/assets/styles.css`), and one vanilla JS file (`ui/src/main.js`) — no build step, no
framework, no bundler. It talks to the [[Tauri Backend]] exclusively through Tauri's IPC bridge.
See [[Home]] for how it fits into the overall data flow.

## Talking to the backend

```js
const invoke = (cmd, args) => window.__TAURI__.core.invoke(cmd, args);
```

`window.__TAURI__` is injected by Tauri because `tauri.conf.json` sets `"withGlobalTauri":
true`. Every backend call in `main.js` goes through this one `invoke` helper — there is no
other transport (no fetch, no websockets) between UI and backend.

## Page structure (`index.html`)

One `.app` container with a header (brand + status pill) and a scrollable `.content` of cards:

1. **Profile** card — profile `<select>`, per-server `<select>` (for switching the active
   outbound within a profile), Import/Delete buttons.
2. **Connect** card — the big Connect/Disconnect toggle button plus live up/down speed and
   totals.
3. **Split tunneling** card — three mode radio buttons (Off/Exclude/Include, matching
   [[Split Tunneling|`SplitMode`]]), the current rule list with remove buttons, "+ Add app" and
   "+ Add domain" controls.
4. **Settings** card — MTU number input, "Launch at login" checkbox, Save button.
5. **Logs** card — collapsible panel showing a polled tail of engine logs.

Plus two modals: **Import profile** (paste JSON or share links) and **Add app** (filterable list
of running processes).

## State and polling (`main.js`)

A single in-memory `state` object holds `profiles`, `activeProfileId`, `split`, `settings`, and
`lastStatus`. On `DOMContentLoaded`, `init()` loads profiles, split config, and settings once,
then starts polling:

- **`pollStatus()`** runs every 1500ms (`setInterval`): calls `status`, and if the engine state
  is `"running"`, also calls `traffic` and updates the speed/total displays. This is why the UI
  is "eventually consistent" with the backend rather than push-driven — there's no event/
  subscription mechanism, just polling.
- **`pollLogs()`** runs every 2000ms, but **only while the Logs panel is expanded** (started/
  stopped in the `logs-toggle` click handler) — logs aren't polled when the panel is collapsed.

## Every `invoke()` call, mapped to its command

| UI action | `invoke(...)` | Backend command (see [[Tauri Backend]]) |
|---|---|---|
| Page load | `list_profiles` | `list_profiles` |
| Page load | `get_split` | `get_split` |
| Page load | `get_settings` | `get_settings` |
| Status poll (1.5s) | `status` | `status` |
| Status poll, if running | `traffic` | `traffic` |
| Change profile dropdown | `set_active_profile` | `set_active_profile` |
| Change server dropdown | `switch_outbound` | `switch_outbound` |
| Click Delete profile | `delete_profile` | `delete_profile` |
| Confirm Import modal | `import_profile` | `import_profile` |
| Click Connect/Disconnect | `connect` or `disconnect` | `connect` / `disconnect` |
| Change split-mode radio | `set_split_mode` | `set_split_mode` |
| Add domain | `add_split_rule` (kind: `domain_suffix`) | `add_split_rule` |
| Pick app in Add-app modal | `add_split_rule` (kind: `process`) | `add_split_rule` |
| Click rule's ✕ | `remove_split_rule` | `remove_split_rule` |
| Open Add-app modal | `list_running_processes` | `list_running_processes` |
| Click Save settings | `set_settings` | `set_settings` |
| Logs panel expanded (2s) | `logs` (n: 200) | `logs` |

Notice `list_profiles`/`get_split`/`get_settings` are only called once at startup plus after
mutations that need a fresh copy (e.g. `import_profile` → `loadProfiles()`,
`delete_profile` → `loadProfiles()`) — the UI otherwise mutates its local `state` object
optimistically (e.g. `state.split.mode = input.value` right after `set_split_mode` succeeds)
rather than re-fetching.

## Error handling

Every `invoke()` call is wrapped in `try/catch`; failures go through `showError()`, which is
deliberately minimal — `console.error` plus `window.alert` — per the project's "no CDN/external
toast library" constraint (the goal is zero external dependencies in the frontend).

## See also

- [[Tauri Backend]] — every command this page calls, and what each does server-side.
- [[Split Tunneling]] — the mode/rule model behind the Split tunneling card.
- [[Building and Running]] — how `frontendDist` (`../ui`) is wired into the Tauri build.
- [[Architecture Overview]] — where the UI sits in the overall data flow.
