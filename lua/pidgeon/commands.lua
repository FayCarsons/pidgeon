local M = {}

function M.setup(config)
  local client = require('pidgeon.client')
  
  vim.api.nvim_create_user_command('PidgeonConnect', function()
    client.connect 
  end, {})

  vim.api.nvim_create_user_command('PidgeonDisconnect', function()
    client.disconnect()
  end, {})

  vim.api.nvim_create_user_command('PidgeonSend', function(opts)
    client.send(opts.args)
  end, {nargs=1})

  vim.api.nvim_create_user_command('PidgeonStatus', function()
    local status = client.isConnected() and 'Connected' or 'Disconnected'
    vim.notify('Pidgeon status: ' .. status, vim.log.levels.INFO)
  end, {})

  if config.keymaps.sendLine then 
    vim.keymap.set('n', config.keymaps.sendLine, function() 
      local line = vim.api.nvim_get_current_line()
      client.send(line)
    end, {desc='send current line'})
  end

  if config.keymaps.sendSelection then 
    vim.keymap.set('v', config.keymaps.sendSelection, function() 
      local startPos = vim.fn.getpos("'<")
      local endPos = vim.fn.getpos("'>")
      local lines = vim.api.nvim_buf_et_lines(0, startPos[2] - 1, endPos[2], false)
      local code = table.concat(lines, '\n')
      client.send(code)
    end, {desc='send selection'})
  end

  if config.keymaps.sendBuffer then 
    vim.keymap.set('n', config.keymaps.sendBuffer, function() 
      local lines = vim.api.nvim_buf_get_lines(0, 0, -1, false)
      local code table.concat(lines, '\n')
      client.send(code)
    end, {desc='send buffer'})
  end
end

return M
