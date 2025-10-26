# A Carrier Pidgeon for Monome Crow
This is a simple REPL, script loader, and server for the Monome Crow. 
*Currently a work in progress, expect bugs!*

The CLI has three sub-commands:
- File: upload a Lua script
- Repl: Crow REPL (still lacking some niceties)
- Remote: Starts a server which allows one (1) client to connect and send 
  (length-prefixed) chunks of Lua code to Crow. If Crow responds, the server 
  will pass that response along as well. Default port is 6666.

There is also a plug-n-play Neovim plugin in this repo which can be used to send 
Lua expressions and visual selections to Crow, with any responses displayed 
inline.

It can be installed with LazyVim:
```lua
return {
  'FayCarsons/pidgeon.nvim',
  config = function()
    require('pidgeon').setup{
      port = 6666,
      keymaps = {
        sendExpr = '<leader>ce',       -- Send expression under cursor
        sendBuffer = '<leader>ca',     -- Send entire buffer
        sendSelection = '<leader>cv',  -- Send visual selection
      }
    }
  end
}
```

Or with Packer:
```lua
use {
  'FayCarsons/pidgeon.nvim',
  config = function()
    require('pidgeon').setup{
      port = 6666,
      keymaps = {
        sendExpr = '<leader>ce',       -- Send expression under cursor
        sendBuffer = '<leader>ca',     -- Send entire buffer
        sendSelection = '<leader>cv',  -- Send visual selection
      }
    }
  end
}
```

Really any package manager should work but if its not one of those you're on your own I'm afraid
