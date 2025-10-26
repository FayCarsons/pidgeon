-- client.lua: TCP client for length-prefixed JSON communication
local M = {}

local uv = vim.uv or vim.loop -- vim.uv is the modern name, vim.loop is older

local function pack_be_u32(value)
  return string.char(
    bit.rshift(value, 24),
    bit.band(bit.rshift(value, 16), 0xFF),
    bit.band(bit.rshift(value, 8), 0xFF),
    bit.band(value, 0xFF)
  )
end

-- Big-endian unpack
local function unpack_be_u32(bytes)
  local b1, b2, b3, b4 = bytes:byte(1, 4)
  return bit.lshift(b1, 24) + bit.lshift(b2, 16) + bit.lshift(b3, 8) + b4
end

---@class Client
---@field private _socket uv.uv_tcp_t|nil
---@field private _buffer string
---@field private _connected boolean
---@field private _connecting boolean
---@field private _host string
---@field private _port integer
---@field private _on_message function|nil
---@field private _on_error function|nil
---@field private _on_connect function|nil
---@field private _on_disconnect function|nil

local Client = {}
Client.__index = Client

--- Create a new TCP client
---@param host string The hostname or IP address to connect to
---@param port integer The port number to connect to
---@param opts? table Optional configuration
---   - on_message: function(data: table) Called when a complete JSON message is received
---   - on_error: function(err: string) Called when an error occurs
---   - on_connect: function() Called when connection is established
---   - on_disconnect: function() Called when connection is closed
---@return Client
function M.new(host, port, opts)
  opts = opts or {}

  local self = setmetatable({
    _socket = nil,
    _buffer = "",
    _connected = false,
    _connecting = false,
    _host = host,
    _port = port,
    _on_message = opts.on_message,
    _on_error = opts.on_error,
    _on_connect = opts.on_connect,
    _on_disconnect = opts.on_disconnect,
  }, Client)

  return self
end

--- Connect to the server
---@param callback? function Optional callback when connection completes
function Client:connect(callback)
  if self._connected then
    if callback then
      callback(nil) -- Already connected, no error
    end
    return
  end

  if self._connecting then
    if callback then
      callback("already connecting")
    end
    return
  end

  self._connecting = true
  self._socket = uv.new_tcp()

  uv.tcp_connect(self._socket, self._host, self._port, function(err)
    self._connecting = false

    if err then
      self._connected = false
      self:_handle_error("connection error: " .. err)
      if callback then
        callback(err)
      end
      return
    end

    self._connected = true

    -- Start reading from the socket
    self:_start_reading()

    -- Trigger connect callback
    if self._on_connect then
      vim.schedule(function()
        self._on_connect()
      end)
    end

    if callback then
      callback(nil)
    end
  end)
end

--- Start reading from the socket
function Client:_start_reading()
  if not self._socket then
    return
  end

  uv.read_start(self._socket, function(err, chunk)
    if err then
      self:_handle_error("read error: " .. err)
      self:disconnect()
      return
    end

    if not chunk then
      -- EOF
      self:disconnect()
      return
    end

    -- Add chunk to buffer and process messages
    self._buffer = self._buffer .. chunk
    self:_process_buffer()
  end)
end

--- Process the buffer to extract complete messages
function Client:_process_buffer()
  while #self._buffer >= 4 do
    local msg_len = unpack_be_u32(self._buffer:sub(1, 4))

    -- Check if we have the complete message
    if #self._buffer < 4 + msg_len then
      -- Not enough data yet, wait for more
      break
    end

    -- Extract the JSON message
    local json_str = self._buffer:sub(5, 4 + msg_len)
    self._buffer = self._buffer:sub(5 + msg_len)

    -- Decode JSON and trigger callback
    local success, result = pcall(vim.json.decode, json_str)

    if success then
      if self._on_message then
        -- Schedule to run in main event loop to avoid re-entrancy issues
        vim.schedule(function()
          self._on_message(result)
        end)
      end
    else
      self:_handle_error("JSON decode error: " .. tostring(result))
    end
  end
end

--- Send a message to the server
---@param data table The data to send (will be JSON encoded)
---@param callback? function Optional callback(err: string|nil) when write completes
function Client:send(data, callback)
  if not self._connected or not self._socket then
    local err = "Not connected"
    if callback then
      callback(err)
    end
    self:_handle_error(err)
    return
  end

  -- Encode to JSON
  local success, json_str = pcall(vim.json.encode, data)
  if not success then
    local err = "JSON encode error: " .. tostring(json_str)
    if callback then
      callback(err)
    end
    self:_handle_error(err)
    return
  end

  -- Create length-prefixed message (4-byte big-endian length + JSON)
  local msg_len = #json_str
  local prefix = pack_be_u32(msg_len)
  local message = prefix .. json_str

  -- Send the message
  uv.write(self._socket, message, function(err)
    if err then
      self:_handle_error("Write error: " .. err)
    end

    if callback then
      callback(err)
    end
  end)
end

--- Check if the client is connected
---@return boolean
function Client:is_connected()
  return self._connected
end

--- Disconnect from the server
function Client:disconnect()
  if not self._socket then
    return
  end

  local was_connected = self._connected
  self._connected = false
  self._connecting = false

  -- Stop reading
  if not uv.is_closing(self._socket) then
    uv.read_stop(self._socket)
    uv.close(self._socket)
  end

  self._socket = nil
  self._buffer = ""

  -- Trigger disconnect callback only if we were actually connected
  if was_connected and self._on_disconnect then
    vim.schedule(function()
      self._on_disconnect()
    end)
  end
end

local x = 2 + 2

--- Handle errors
---@param err string
function Client:_handle_error(err)
  if self._on_error then
    vim.schedule(function()
      self._on_error(err)
    end)
  end
end

--- Get connection info
---@return table|nil
function Client:get_info()
  if not self._connected or not self._socket then
    return nil
  end

  return {
    host = self._host,
    port = self._port,
    connected = self._connected,
  }
end

return M
