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

  require('pidgeon.commands').setup(M.config)

  if M.config.autoConnect then 
    vim.defer_fn(function 
      require('pidgeon.client').connect()
    end, 1000)
  end
end

return M
