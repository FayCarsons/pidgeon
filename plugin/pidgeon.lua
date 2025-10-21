local client = nil

function or_require()
  if not client then 
    client = require('pidgeon.client')
  end

  return client
end

vim.api.nvim_create_user_command('PidgeonConnect', function()
  or_require().connect 
end, {})

vim.api.nvim_create_user_command('PidgeonDisconnect', function()
  or_require().disconnect()
end, {})

vim.api.nvim_create_user_command('PidgeonSend', function(opts)
  or_require().send(opts.args)
end, { nargs = 1 })

vim.api.nvim_create_user_command('PidgeonStatus', function()
  local status = or_require().isConnected() and 'Connected' or 'Disconnected'

  vim.notify('Pidgeon status: ' .. status, vim.log.levels.INFO)
end, {})
