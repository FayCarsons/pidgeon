vim.api.nvim_create_user_command('PidgeonConnect', function()
  require('pidgeon.client').connect() 
end, { desc = 'connect to pidgeon server' })

vim.api.nvim_create_user_command('PidgeonDisconnect', function()
  require('pidgeon.client').disconnect()
end, { desc = 'close connection to pidgeon server' })

vim.api.nvim_create_user_command('PidgeonSend', function(opts)
  require('pidgeon.treesitter').sendNearest()
end, { desc = 'send the expression under the cursor to pidgeon' })

vim.api.nvim_create_user_command('PigeonSendBuf', function(opts)
  require('pidgeon.client').sendCurrentBuf()
end, { desc = 'send the current buffer to crow' })

vim.api.nvim_create_user_command('PidgeonStatus', function()
  local status = require('pidgeon.client').isConnected() and 'Connected' or 'Disconnected'

  vim.notify('pidgeon status: ' .. status, vim.log.levels.INFO)
end, { desc = 'get the status of the pidgeon server' })
