-- Visible-screen capture for `rex do -e <code> --json --args <json>`; unset
-- REX_SESSION, REX_BLOCK and REX_CLIENT in the child.
-- Arguments: {session_id, block_id, budget}. `budget` caps the content size,
-- so the reply size does not grow with the block's scrollback.
-- Returns {rows, content, partial}. Without `unwrap`, each content line is one
-- screen row, so the caller keeps the last `rows` lines. When `partial` is
-- true, content is the tail of the output starting at an arbitrary byte, and
-- the caller drops its first (cut) line.

local args = rex.args
local session_id = args.session_id
local block_id = args.block_id
local budget = args.budget

-- Creator-method calls return nil unless this connection is attached.
rex.call("session.attach", {session_id = session_id})

local function terminal(method, call_args)
    local reply = rex.call("com.superlogical.terminal." .. method, {
        session_id = session_id,
        block_id = block_id,
        args = call_args,
    })
    if type(reply) ~= "table" then
        error("Rex terminal." .. method .. " returned no result for " .. block_id)
    end
    return reply
end

local rows = terminal("size", {}).rows
local first = terminal("format", {format = "vt", trim = true, max_bytes = budget})
if first.id == nil then
    return {rows = rows, content = first.content or "", partial = false}
end

-- `total` and `offset` count raw bytes, but `max_bytes` counts JSON-encoded
-- bytes (ESC becomes `\u001b`), so one read can stop short of the end. Keep
-- reading until EOF; the tail is then at most `budget` raw bytes.
local offset = math.max(0, first.total - budget)
local parts = {}
for _ = 1, 64 do
    local chunk = terminal("blob_read", {id = first.id, offset = offset, max_bytes = budget})
    local data = chunk.data or ""
    parts[#parts + 1] = data
    offset = offset + #data
    if chunk.eof or #data == 0 or offset >= first.total then
        break
    end
end
return {rows = rows, content = table.concat(parts), partial = true}
