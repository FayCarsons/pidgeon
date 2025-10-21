local M = {}

M.config = {
  pidgeonURL = "ws://localhost:6666/connect",
  autoConnect = true,
  keymaps = {
    sendLine = "<leader>cl",
    sendSelection = "<leader>cs",
    sendBuffer = "<leader>cb",
  }
}

function M.setup(opts) 
  M.config = vim.tbl_deep_extend('force', M.config, opts or {})

  local client = require('pidgeon.client')

  if M.config.keymaps.sendLine then 
    vim.keymap.set('n', M.config.keymaps.sendLine, function() 
      local line = vim.api.nvim_get_current_line()
      client.send(line)
    end, {desc='send current line'})
  end

  if M.config.keymaps.sendSelection then 
    vim.keymap.set('v', M.config.keymaps.sendSelection, function() 
      local startPos = vim.fn.getpos("'<")
      local endPos = vim.fn.getpos("'>")
      local lines = vim.api.nvim_buf_get_lines(0, startPos[2] - 1, endPos[2], false)
      local code = table.concat(lines, '\n')
      client.send(code)
    end, {desc='send selection'})
  end

  if M.config.keymaps.sendBuffer then 
    vim.keymap.set('n', M.config.keymaps.sendBuffer, function() 
      client.sendBuf()
    end, {desc='send buffer'})
  end

  if M.config.keymaps.sendNearestExpression then 
    vim.keymap.set('n', M.config.keymaps.sendNearestExpression, function()
      require('pidgeon.treesitter').sendNearest()
    end, { desc = 'send nearest expression' })
  end

  if M.config.autoConnect then 
    vim.defer_fn(function() 
      require('pidgeon.client').connect()
    end, 1000)
  end
end

return M
