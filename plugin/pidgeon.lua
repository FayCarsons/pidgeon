vim.api.nvim_create_user_command('PidgeonConnect', function()
  require('pidgeon').connect()
end, { desc = 'connect to pidgeon server' })

vim.api.nvim_create_user_command('PidgeonDisconnect', function()
  require('pidgeon').disconnect()
end, { desc = 'close connection to pidgeon server' })

vim.api.nvim_create_user_command('PidgeonSend', function(opts)
  require('pidgeon').sendExpr()
end, { desc = 'send the expression under the cursor to pidgeon' })

vim.api.nvim_create_user_command('PidgeonSendBuf', function(opts)
  require('pidgeon').sendBuffer()
end, { desc = 'send the current buffer to crow' })

vim.api.nvim_create_user_command('PidgeonStatus', function()
  require('pidgeon').check()
end, { desc = 'get the status of the pidgeon server' })
