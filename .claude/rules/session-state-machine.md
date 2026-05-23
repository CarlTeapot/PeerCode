# Session State Machine

`AppRole` in `tauri-app/src-tauri/src/state/appstate.rs` is a strict FSM. All transitions are validated — invalid ones return `TransitionError` at runtime.

## Allowed transitions

```
Undecided → Starting   →  Host { … }
                       →  Guest { … }
                       →  Undecided   (rollback)
Host | Guest → Undecided
```

## How to initiate a session

Always use the RAII guard. Never call `transition_role(Starting)` directly:

```rust
let guard = state.begin_session(app.clone())?;  // Undecided → Starting
// do async setup — guard auto-rolls back to Undecided if dropped (error / early return)
state.complete_host(guard, room_id, lan_url, public_url, local_room_url, public_room_url)?;
// or
state.complete_guest(guard, room_id, server_url)?;
```

## How to end a session

```rust
state.transition_role(AppRole::Undecided)?;  // Host | Guest → Undecided
```

## Rules

- **Never** lock `AppState.role` directly — the field is private; use `AppState` methods.
- **Never** call `transition_role(Starting)` directly — use `begin_session` so the guard provides automatic rollback.
- **No explicit rollbacks** in error paths — dropping `StartingGuard` handles it.
- `StartingGuard` has no public methods. Complete it only via `complete_host` / `complete_guest` on `AppState`.
