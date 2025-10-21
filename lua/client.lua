local M = {}

local client = nil
local isConnected = false

local function checkPidgeon()
  local handle = io.popen("curl -s http://localhost:6666/check 2>/dev/null")
  if not handle then
    return false
  end

  local result = handle:read('*a')
  handle:close()

  local ok, data = pcall(vim.json.decode, result)
  return ok and data
end

function M.connect()
  local status = checkPidgeon()
  
  if not status then 
    vim.notify('Pidgeon is busy with another connection')
    return false
  end

  local WebsocketClient = require('websocket.client').WebsocketClient

  client = WebsocketClient.new{
    connect_addr = require('pidgeon').config.pidgeonURL,

    on_message = function(self, message)
      vim.notify(message, vim.log.levels.INFO, { title = 'Pidgeon' })
    end,

    on_error = function(self, err)
      vim.notify('Crow error: ' .. vim.inspect(err), vim.log.levels.ERROR)
    end
  }

  client:try_connect()
  return true
end

function M.disconnect()
  if client then 
    client:try_disconnect()
    client = nil
    isConnected = false
  end
end

function M.send(code)
  if not (isConnected and client) then
    vim.notify('Not connected to Pidgeon. Run :PidgeonConnect', vim.log.levels.WARN)
    return false
  end

  client:try_send_data(code)
  return true
end

function M.isConnected()
  return isConnected
end

return M
