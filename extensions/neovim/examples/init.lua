# Example Neovim Configuration

This directory contains example configurations for using CodeInput with Neovim.

## Minimal Setup

```lua
-- init.lua
require("codeinput").setup()
```

## With Status Line

Using the built-in status line:

```lua
-- init.lua
require("codeinput").setup()

-- Add to status line
vim.o.statusline = "%<%f %h%m%r%=%{v:lua.require('codeinput.statusline').get_status()} %-14.(%l,%c%V%) %P"
```

## With lualine.nvim

```lua
-- init.lua with lualine
require("codeinput").setup()

require('lualine').setup {
  sections = {
    lualine_c = {
      'filename',
      { 
        function() 
          return require('codeinput.statusline').get_status() 
        end,
        cond = function()
          local status = require('codeinput.statusline').get_status()
          return status ~= "" and status ~= nil
        end,
        color = { gui = "bold" },
      }
    }
  }
}
```

## Advanced Configuration

```lua
-- init.lua with custom settings
require("codeinput").setup({
  binary_path = "/usr/local/bin/ci-lsp",  -- Custom binary path
  cache_file = ".codeowners.cache",        -- Cache file name
  show_diagnostics = true,                 -- Show diagnostics
  show_in_statusline = true,               -- Enable status line
})

-- Key mappings
vim.api.nvim_set_keymap('n', '<leader>ci', ':CodeInputInfo<CR>', { noremap = true, silent = true })
vim.api.nvim_set_keymap('n', '<leader>cr', ':CodeInputRefresh<CR>', { noremap = true, silent = true })
```

## With nvim-tree

Show ownership info in file tree (requires nvim-tree):

```lua
-- This is a more advanced example
local codeinput = require("codeinput")

require("nvim-tree").setup {
  view = {
    float = {
      enable = true,
    }
  },
  renderer = {
    icons = {
      show = {
        git = true,
      }
    }
  }
}
```
