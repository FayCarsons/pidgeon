local M = {}

M.config = {
  host = "127.0.0.1",
  port = 6666,
  keymaps = {
    sendExpr = "<leader>ce",
    sendBuffer = "<leader>ca",
    sendSelection = '<leader>cv',
  },
  inline_results = {
    enabled = true,
    prefix = " => ",
    highlight = "Comment",
    max_length = 100,
  }
}

---@type Client | nil
local client = nil

-- Namespace for inline results
local inline_ns = vim.api.nvim_create_namespace('pidgeon_inline_results')

-- Pending requests (to track which line to display results on)
local pending_requests = {}
local request_id_counter = 0

local function ensureConnected()
  if not client or not client:is_connected() then
    vim.notify('pidgeon not connected, run :PidgeonConnect', vim.log.levels.WARN)
    return false
  end
  return true
end

local function clearInlineResults(bufnr)
  bufnr = bufnr or vim.api.nvim_get_current_buf()
  vim.api.nvim_buf_clear_namespace(bufnr, inline_ns, 0, -1)
end

local function showInlineResult(bufnr, line, result_text)
  if not M.config.inline_results.enabled then
    return
  end

  bufnr = bufnr or vim.api.nvim_get_current_buf()

  -- Convert result to string if needed
  if type(result_text) ~= 'string' then
    result_text = vim.inspect(result_text)
  end

  -- Remove newlines for inline display
  result_text = result_text:gsub('\n', ' ')

  -- Truncate if too long
  if #result_text > M.config.inline_results.max_length then
    result_text = result_text:sub(1, M.config.inline_results.max_length) .. "..."
  end

  local virt_text = M.config.inline_results.prefix .. result_text

  vim.api.nvim_buf_set_extmark(bufnr, inline_ns, line, 0, {
    virt_text = { { virt_text, M.config.inline_results.highlight } },
    virt_text_pos = 'eol',
  })
end

local function handleResponse(data)
  if not data then return end

  if data.status == 'Success' then
    local request_id = data.request_id
    local request_info = pending_requests[request_id]

    if request_info then
      pending_requests[request_id] = nil
      local result_str = data.contents

      -- Show inline if we have context
      if request_info.bufnr and request_info.line then
        showInlineResult(request_info.bufnr, request_info.line, result_str)
      end

      -- Also show in notification for very short results or if inline is disabled
      if not M.config.inline_results.enabled or #result_str < 20 then
        vim.notify('result: ' .. result_str, vim.log.levels.INFO)
      end
    else
      -- No context, just show notification
      vim.notify('result: ' .. data.contents, vim.log.levels.INFO)
    end
  elseif data.status == 'Failure' then
    local request_id = data.request_id
    local request_info = pending_requests[request_id]
    local error_msg = data.contents or 'unknown error'

    if request_info then
      pending_requests[request_id] = nil

      -- Show error inline with special formatting
      if request_info.bufnr and request_info.line and M.config.inline_results.enabled then
        vim.api.nvim_buf_set_extmark(request_info.bufnr, inline_ns, request_info.line, 0, {
          virt_text = { { M.config.inline_results.prefix .. "ERROR: " .. error_msg, "ErrorMsg" } },
          virt_text_pos = 'eol',
        })
      end
    end

    vim.notify('error: ' .. error_msg, vim.log.levels.ERROR)
  elseif data.status == 'Affirm' then
    vim.notify('server ready', vim.log.levels.INFO)
  end
end

local function sendCode(code, context)
  if not ensureConnected() then
    return
  end

  request_id_counter = request_id_counter + 1
  local request_id = request_id_counter

  -- Store context for this request
  if context then
    pending_requests[request_id] = context
  end

  client:send({
    status = 'Success',
    request_id = request_id,
    contents = code,
  }, function(err)
    if err then
      vim.notify('failed to send: ' .. err, vim.log.levels.ERROR)
      pending_requests[request_id] = nil
    end
  end)
end

function M.connect()
  if client and client:is_connected() then
    vim.notify('already connected', vim.log.levels.WARN)
    return
  end

  local Client = require('pidgeon.client')

  client = Client.new(M.config.host, M.config.port, {
    on_connect = function()
      vim.notify('pidgeon connected to ' .. M.config.host .. ':' .. M.config.port, vim.log.levels.INFO)
    end,

    on_disconnect = function()
      vim.notify('pidgeon disconnected', vim.log.levels.INFO)
    end,

    on_message = handleResponse,

    on_error = function(err)
      vim.notify('pidgeon error: ' .. err, vim.log.levels.ERROR)
    end,
  })

  client:connect(function(err)
    if err then
      vim.notify('failed to connect to pidgeon: ' .. err, vim.log.levels.ERROR)
      client = nil
    else
      -- Send Start message to initiate session
      client:send({
        status = 'Start'
      }, function(start_err)
        if start_err then
          vim.notify('failed to start session: ' .. start_err, vim.log.levels.ERROR)
        end
      end)
    end
  end)
end

function M.disconnect()
  if not client then
    vim.notify('not connected', vim.log.levels.WARN)
    return
  end

  client:disconnect()
  client = nil
  clearInlineResults()
end

function M.check()
  if not client or not client:is_connected() then
    vim.notify('not connected', vim.log.levels.WARN)
    return
  end

  -- Create a temporary client to check if server is busy with persistent connection
  local Client = require('pidgeon.client')
  local check_client = Client.new(M.config.host, M.config.port, {
    on_message = function(data)
      if data.status == 'Affirm' then
        vim.notify('server available for persistent connection', vim.log.levels.INFO)
      elseif data.status == 'Failure' and data.contents == 'BUSY' then
        vim.notify('server busy with another persistent connection', vim.log.levels.WARN)
      end
      check_client:disconnect()
    end,
    on_error = function(err)
      vim.notify('check failed: ' .. err, vim.log.levels.ERROR)
      check_client:disconnect()
    end
  })

  check_client:connect(function(err)
    if err then
      vim.notify('failed to connect for check: ' .. err, vim.log.levels.ERROR)
    else
      check_client:send({
        status = 'Check'
      }, function(send_err)
        if send_err then
          vim.notify('failed to send check: ' .. send_err, vim.log.levels.ERROR)
          check_client:disconnect()
        end
      end)
    end
  end)
end

function M.sendSelection()
  if not ensureConnected() then return end

  local startPos = vim.fn.getpos("'<")
  local endPos = vim.fn.getpos("'>")
  local bufnr = vim.api.nvim_get_current_buf()
  local lines = vim.api.nvim_buf_get_lines(bufnr, startPos[2] - 1, endPos[2], false)

  if #lines == 0 then
    vim.notify('no selection', vim.log.levels.WARN)
    return
  end

  if #lines == 1 then
    lines[1] = lines[1]:sub(startPos[3], endPos[3])
  else
    lines[1] = lines[1]:sub(startPos[3])
    lines[#lines] = lines[#lines]:sub(1, endPos[3])
  end

  local code = table.concat(lines, '\n')

  clearInlineResults(bufnr)

  sendCode(code, {
    bufnr = bufnr,
    line = endPos[2] - 1, -- 0-indexed, show at last line of selection
  })
end

function M.sendBuffer()
  if not ensureConnected() then return end

  local bufnr = vim.api.nvim_get_current_buf()
  local lines = vim.api.nvim_buf_get_lines(bufnr, 0, -1, false)
  local code = table.concat(lines, '\n')

  clearInlineResults(bufnr)

  sendCode(code, {
    bufnr = bufnr,
    line = #lines - 1, -- Show at last line
  })
end

function M.sendExpr()
  if not ensureConnected() then return end

  local PidgeonTreesitter = require('pidgeon.treesitter')
  local result = PidgeonTreesitter.getNearest()

  if not result then
    return
  end

  local bufnr = vim.api.nvim_get_current_buf()

  clearInlineResults(bufnr)

  local clearHighlight = PidgeonTreesitter.highlight(result)

  sendCode(result.text, {
    bufnr = bufnr,
    line = result.range.endRow,
  })

  -- Clear highlight after a delay
  vim.defer_fn(function()
    if clearHighlight then
      clearHighlight()
    end
  end, 500)
end

function M.clearResults()
  clearInlineResults()
  vim.notify('cleared inline results', vim.log.levels.INFO)
end

function M.setup(opts)
  M.config = vim.tbl_deep_extend('force', M.config, opts or {})

  vim.api.nvim_create_user_command('PidgeonConnect', M.connect, {
    desc = 'connect to pidgeon server'
  })

  vim.api.nvim_create_user_command('PidgeonDisconnect', M.disconnect, {
    desc = 'disconnect from pidgeon server'
  })

  vim.api.nvim_create_user_command('PidgeonCheck', M.check, {
    desc = 'check pidgeon connection status'
  })

  vim.api.nvim_create_user_command('PidgeonClearResults', M.clearResults, {
    desc = 'clear inline results'
  })

  if M.config.keymaps.sendSelection then
    vim.keymap.set('v', M.config.keymaps.sendSelection, M.sendSelection, {
      desc = 'pidgeon: send selection'
    })
  end

  if M.config.keymaps.sendBuffer then
    vim.keymap.set('n', M.config.keymaps.sendBuffer, M.sendBuffer, {
      desc = 'pidgeon: send buffer'
    })
  end

  if M.config.keymaps.sendExpr then
    vim.keymap.set('n', M.config.keymaps.sendExpr, M.sendExpr, {
      desc = 'pidgeon: send nearest evaluable expression'
    })
  end
end

return M
