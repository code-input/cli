-- Minimal test configuration for CodeInput Neovim plugin
-- Run with: nvim -u test_init.lua

vim.opt.runtimepath:append(".")
vim.opt.runtimepath:append("../lua")

require("codeinput").setup()

vim.o.statusline = "%<%f %h%m%r%=%{v:lua.require('codeinput.statusline').get_status()} %-14.(%l,%c%V%) %P"

vim.api.nvim_set_keymap('n', '<leader>ci', ':CodeInputInfo<CR>', { noremap = true, silent = true })
vim.api.nvim_set_keymap('n', '<leader>cr', ':CodeInputRefresh<CR>', { noremap = true, silent = true })

print("CodeInput Neovim plugin loaded!")
print("Commands available:")
print("  :CodeInputInfo - Show ownership info")
print("  :CodeInputRefresh - Refresh cache")
print("  <leader>ci - Show info (keymap)")
print("  <leader>cr - Refresh cache (keymap)")
