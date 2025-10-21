local M = {}

local client = nil
local isConnected = false

local function checkPidgeon()
  -- Curl endpoint to see if pidgeon is available
  local handle = io.popen("curl -s http://localhost:6666/check 2>/dev/null")

  if not handle then
    return false
  end

  local result = handle:read('*a')
  handle:close()

  if result == "" then return false end

  local ok, data = pcall(vim.json.decode, result)
  return ok and data
end

function M.connect()
  local status = checkPidgeon()
  
  if not status then 
    vim.notify('Pidgeon is busy with another connection')
    return false
  end

  local ok, WebsocketClient = pcall(require, 'websocket.client')
  if not ok then 
    vim.notify('websocket.nvim not installed', vim.log.levels.ERROR)
    return false
  end


  client = WebsocketClient.WebsocketClient.new{
    connect_addr = require('pidgeon').config.pidgeonURL,

    on_message = function(self, message)
      vim.notify(message, vim.log.levels.INFO, { title = 'Pidgeon' })
    end,

    on_connect = function(self) 
      isConnected = true
      vim.notify('connected to pidgeon', vim.log.levels.INFO)
    end,

    on_disconnect = function(self)
      isConnected = false
      vim.notify('pidgeon disconnected', vim.log.levels.INFO)
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
  if not isConnected or not client then
    vim.notify('not connected to pidgeon - run :PidgeonConnect', vim.log.levels.WARN)
    return false
  end

  client:try_send_data(code)
  return true
end

function M.isConnected()
  return isConnected
end

return M
