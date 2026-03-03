local M = {}

local config = {
  binary_path = "ci",
  cache_file = ".codeowners.cache",
  show_diagnostics = true,
  show_in_statusline = true,
}

local client_id = nil

local function get_root_dir()
  local fname = vim.api.nvim_buf_get_name(0)
  if fname == "" then
    return nil
  end
  
  local root = vim.fs.dirname(vim.fs.find({ "CODEOWNERS", ".git" }, { upward = true, path = fname })[1])
  return root
end

local function start_lsp()
  if client_id then
    return
  end
  
  local root_dir = get_root_dir()
  if not root_dir then
    return
  end
  
  client_id = vim.lsp.start_client({
    name = "codeinput",
    cmd = { config.binary_path, "lsp" },
    root_dir = root_dir,
    on_exit = function(code, signal, client_id)
      client_id = nil
      if code ~= 0 then
        vim.notify("CodeInput LSP exited with code " .. code, vim.log.levels.ERROR)
      end
    end,
    on_error = function(code, err)
      vim.notify("CodeInput LSP error: " .. err, vim.log.levels.ERROR)
    end,
  })
end

local function attach_to_buffer(bufnr)
  bufnr = bufnr or vim.api.nvim_get_current_buf()
  
  if not client_id then
    start_lsp()
  end
  
  if client_id then
    vim.lsp.buf_attach_client(bufnr, client_id)
  end
end

function M.setup(opts)
  config = vim.tbl_deep_extend("force", config, opts or {})
  
  local lsp_group = vim.api.nvim_create_augroup("CodeInputLSP", { clear = true })
  
  vim.api.nvim_create_autocmd("FileType", {
    group = lsp_group,
    pattern = "*",
    callback = function()
      local root = get_root_dir()
      if root then
        attach_to_buffer()
      end
    end,
  })
  
  vim.api.nvim_create_user_command("CodeInputInfo", function()
    M.show_info()
  end, { desc = "Show CODEOWNERS info for current file" })
  
  vim.api.nvim_create_user_command("CodeInputRefresh", function()
    M.refresh_cache()
  end, { desc = "Refresh CODEOWNERS cache" })
  
  vim.api.nvim_create_user_command("CodeInputFiles", function()
    require("codeinput.telescope").files()
  end, { desc = "Browse all files with ownership" })
  
  vim.api.nvim_create_user_command("CodeInputOwners", function()
    require("codeinput.telescope").owners()
  end, { desc = "Browse files by owner" })
  
  vim.api.nvim_create_user_command("CodeInputTags", function()
    require("codeinput.telescope").tags()
  end, { desc = "Browse files by tag" })
  
  vim.api.nvim_create_autocmd("BufWritePost", {
    group = lsp_group,
    pattern = "CODEOWNERS",
    callback = function()
      vim.notify("CODEOWNERS file changed, refreshing cache...", vim.log.levels.INFO)
    end,
  })
end

function M.show_info()
  local client = M.get_client()
  if not client then
    vim.api.nvim_echo({{"CodeInput LSP not running", "ErrorMsg"}}, true, {})
    return
  end
  
  local params = vim.lsp.util.make_position_params(0, "utf-16")
  
  client.request("textDocument/hover", params, function(err, result, ctx)
    if err then
      vim.api.nvim_echo({{"Error: " .. err.message, "ErrorMsg"}}, true, {})
      return
    end
    
    if not result or not result.contents then
      vim.api.nvim_echo({{"No CODEOWNERS info for this file", "Normal"}}, true, {})
      return
    end
    
    local lines = {}
    if type(result.contents) == "table" then
      if result.contents.kind == "markdown" then
        for line in result.contents.value:gmatch("[^\r\n]+") do
          table.insert(lines, line)
        end
      else
        for _, item in ipairs(result.contents) do
          if type(item) == "string" then
            table.insert(lines, item)
          elseif item.value then
            table.insert(lines, item.value)
          end
        end
      end
    elseif type(result.contents) == "string" then
      table.insert(lines, result.contents)
    end
    
    -- Clean up markdown formatting
    local clean_lines = {}
    for _, line in ipairs(lines) do
      line = line:gsub("%*%*", "")  -- Remove bold
      line = line:gsub("`", "")      -- Remove code blocks
      if line:match("%S") then       -- Only non-empty lines
        table.insert(clean_lines, line)
      end
    end
    
    vim.api.nvim_echo({{table.concat(clean_lines, " | "), "Normal"}}, true, {})
  end, 0)
end

function M.refresh_cache()
  vim.notify("Refreshing CODEOWNERS cache...", vim.log.levels.INFO)
  
  local clients = vim.lsp.get_clients({ name = "codeinput" })
  if #clients > 0 then
    vim.notify("Cache will be refreshed automatically when CODEOWNERS files change", vim.log.levels.INFO)
  else
    vim.notify("CodeInput LSP not running", vim.log.levels.WARN)
  end
end

function M.get_client()
  if client_id then
    return vim.lsp.get_client_by_id(client_id)
  end
  return nil
end

return M
