-- Read-only Rex snapshot for `rex do -e <code> --json`; unset REX_SESSION and
-- REX_BLOCK in the child. Returns one JSON document:
-- {sessions: [{session_id, label, active_window_id?, windows: [{window_id,
--   label, focused_block_id?, layers: [{layer_id, kind, state?, blocks: [
--     {block_id, label, flavor, process_name?}]}]}]}]}
-- A '?' field is omitted when unavailable; empty lists serialize as {}.
-- Layers and their blocks are already in Rex's order (tiled first, then floats).

local sessions = {}
for _, session in ipairs(rex.call("session.list").sessions or {}) do
    -- Attach only this CLI connection. The attach response already has the
    -- session view, so no separate session.view request is needed.
    local view = rex.call("session.attach", {session_id = session.session_id}).view
    local windows = {}
    for _, window in ipairs(view.windows or {}) do
        local layers = {}
        for _, layer in ipairs(window.layers or {}) do
            local blocks = {}
            for _, block in ipairs(layer.blocks or {}) do
                local entry = {
                    block_id = block.block_id,
                    label = block.label,
                    flavor = block.flavor,
                }
                if block.creator_name == "com.superlogical.terminal" then
                    -- A process can exit (or the block can close) while polling.
                    local ok, process = pcall(rex.call, "com.superlogical.terminal.process", {
                        session_id = session.session_id,
                        block_id = block.block_id,
                    })
                    if ok and type(process) == "table" and type(process.foreground) == "table" then
                        entry.process_name = process.foreground.name
                    end
                end
                blocks[#blocks + 1] = entry
            end
            layers[#layers + 1] = {
                layer_id = layer.layer_id,
                kind = layer.kind,
                state = layer.state,
                blocks = blocks,
            }
        end
        windows[#windows + 1] = {
            window_id = window.window_id,
            label = window.label,
            focused_block_id = window.focused_block_id,
            layers = layers,
        }
    end
    sessions[#sessions + 1] = {
        session_id = session.session_id,
        label = view.label,
        active_window_id = view.active_window_id,
        windows = windows,
    }
end
return {sessions = sessions}
