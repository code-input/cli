local M = {}
local pickers = require("telescope.pickers")
local finders = require("telescope.finders")
local conf = require("telescope.config").values
local actions = require("telescope.actions")
local action_state = require("telescope.actions.state")
local previewers = require("telescope.previewers")

local function path_to_string(path)
  if type(path) == "table" then
    return path[1] or ""
  else
    return tostring(path)
  end
end

local function make_ownership_previewer(ownership_map)
  return previewers.new_buffer_previewer {
    title = "Ownership Details",
    define_preview = function(self, entry)
      local lines = {}
      local path = entry[1]
      local owners_data = ownership_map and ownership_map[path]
      
      if owners_data then
        table.insert(lines, "File: " .. path)
        table.insert(lines, "")
        
        if owners_data.owners and #owners_data.owners > 0 then
          table.insert(lines, "Owners:")
          for _, owner in ipairs(owners_data.owners) do
            table.insert(lines, "  • " .. owner.identifier .. " (" .. owner.owner_type .. ")")
          end
        else
          table.insert(lines, "Owners: (none)")
        end
        
        table.insert(lines, "")
        
        if owners_data.tags and #owners_data.tags > 0 then
          table.insert(lines, "Tags:")
          for _, tag in ipairs(owners_data.tags) do
            local tag_name = type(tag) == "table" and tag[1] or tostring(tag)
            table.insert(lines, "  • #" .. tag_name)
          end
        end
        
        if owners_data.is_unowned then
          table.insert(lines, "")
          table.insert(lines, "⚠️  This file has no CODEOWNERS assignment")
        end
      else
        table.insert(lines, "No ownership data available")
      end
      
      vim.api.nvim_buf_set_lines(self.state.bufnr, 0, -1, false, lines)
      vim.api.nvim_buf_set_option(self.state.bufnr, "filetype", "markdown")
      vim.api.nvim_buf_set_option(self.state.bufnr, "modifiable", false)
    end,
  }
end

function M.files(opts)
  opts = opts or {}
  
  local client = require("codeinput").get_client()
  if not client then
    vim.notify("CodeInput LSP not running", vim.log.levels.WARN)
    return
  end
  
  client.request("workspace/executeCommand", {
    command = "codeinput.listFiles",
    arguments = {},
  }, function(err, result)
    if err then
      vim.notify("Error listing files: " .. err.message, vim.log.levels.ERROR)
      return
    end
    
    if not result or not result.files then
      vim.notify("No files found", vim.log.levels.INFO)
      return
    end
    
    show_files_picker(result.files, opts)
  end)
end

function M.owners(opts)
  opts = opts or {}
  
  local client = require("codeinput").get_client()
  if not client then
    vim.notify("CodeInput LSP not running", vim.log.levels.WARN)
    return
  end
  
  client.request("workspace/executeCommand", {
    command = "codeinput.listOwners",
    arguments = {},
  }, function(err, result)
    if err then
      vim.notify("Error listing owners: " .. err.message, vim.log.levels.ERROR)
      return
    end
    
    if not result or not result.owners then
      vim.notify("No owners found", vim.log.levels.INFO)
      return
    end
    
    show_owners_picker(result.owners, opts)
  end)
end

function M.tags(opts)
  opts = opts or {}
  
  local client = require("codeinput").get_client()
  if not client then
    vim.notify("CodeInput LSP not running", vim.log.levels.WARN)
    return
  end
  
  client.request("workspace/executeCommand", {
    command = "codeinput.listTags",
    arguments = {},
  }, function(err, result)
    if err then
      vim.notify("Error listing tags: " .. vim.inspect(err), vim.log.levels.ERROR)
      return
    end
    
    if not result or not result.tags or #result.tags == 0 then
      vim.notify("No tags found", vim.log.levels.INFO)
      return
    end
    
    show_tags_picker(result.tags, opts)
  end)
end

function show_files_picker(files, opts)
  opts = opts or {}
  
  -- Convert to simple list of file paths
  local file_paths = {}
  local ownership_map = {}
  
  for _, entry in ipairs(files) do
    local path_str = path_to_string(entry.path)
    table.insert(file_paths, path_str)
    ownership_map[path_str] = entry
  end
  
  -- Use telescope's built-in file finder
  local find_files = require("telescope.builtin").find_files
  
  -- Override attach_mappings to add custom previewer
  local original_attach_mappings = opts.attach_mappings
  opts.attach_mappings = function(prompt_bufnr, map)
    if original_attach_mappings then
      original_attach_mappings(prompt_bufnr, map)
    end
    
    actions.select_default:replace(function()
      local selection = action_state.get_selected_entry()
      actions.close(prompt_bufnr)
      vim.cmd("edit " .. vim.fn.fnameescape(selection[1]))
    end)
    
    return true
  end
  
  -- Use find_files but with our file list
  pickers.new(opts, {
    prompt_title = "Files with Ownership",
    finder = finders.new_table {
      results = file_paths,
      entry_maker = require("telescope.make_entry").gen_from_file(opts),
    },
    sorter = conf.file_sorter(opts),
    previewer = make_ownership_previewer(ownership_map),
  }):find()
end

function show_owners_picker(owners, opts)
  pickers.new(opts, {
    prompt_title = "Browse by Owner",
    finder = finders.new_table {
      results = owners,
      entry_maker = function(entry)
        local display = string.format("%s (%d files)", entry.owner.identifier, #entry.files)
        return {
          value = entry,
          display = display,
          ordinal = entry.owner.identifier,
        }
      end,
    },
    sorter = conf.generic_sorter(opts),
    attach_mappings = function(prompt_bufnr, map)
      actions.select_default:replace(function()
        local selection = action_state.get_selected_entry()
        actions.close(prompt_bufnr)
        show_files_for_owner(selection.value.files, selection.value.owner.identifier, opts)
      end)
      return true
    end,
  }):find()
end

function show_tags_picker(tags, opts)
  if not tags or #tags == 0 then
    vim.notify("No tags found", vim.log.levels.INFO)
    return
  end
  
  pickers.new(opts, {
    prompt_title = "Browse by Tag",
    finder = finders.new_table {
      results = tags,
      entry_maker = function(entry)
        local tag_name = type(entry.tag) == "table" and entry.tag[1] or tostring(entry.tag)
        local file_count = entry.files and #entry.files or 0
        local display = string.format("#%s (%d files)", tag_name, file_count)
        
        return {
          value = entry,
          display = display,
          ordinal = tag_name,
        }
      end,
    },
    sorter = conf.generic_sorter(opts),
    attach_mappings = function(prompt_bufnr, map)
      actions.select_default:replace(function()
        local selection = action_state.get_selected_entry()
        actions.close(prompt_bufnr)
        local tag_name = type(selection.value.tag) == "table" and selection.value.tag[1] or tostring(selection.value.tag)
        show_files_for_tag(selection.value.files, tag_name, opts)
      end)
      return true
    end,
  }):find()
end

function show_files_for_owner(files, owner, opts)
  local file_strings = vim.tbl_map(function(f)
    if type(f) == "table" then
      return f[1] or vim.inspect(f)
    else
      return tostring(f)
    end
  end, files)
  
  pickers.new(opts, {
    prompt_title = string.format("Files owned by %s", owner),
    finder = finders.new_table {
      results = file_strings,
    },
    sorter = conf.file_sorter(opts),
    previewer = conf.file_previewer(opts),
    attach_mappings = function(prompt_bufnr, map)
      actions.select_default:replace(function()
        local selection = action_state.get_selected_entry()
        actions.close(prompt_bufnr)
        vim.cmd("edit " .. vim.fn.fnameescape(selection[1]))
      end)
      return true
    end,
  }):find()
end

function show_files_for_tag(files, tag, opts)
  local file_strings = vim.tbl_map(function(f)
    if type(f) == "table" then
      return f[1] or vim.inspect(f)
    else
      return tostring(f)
    end
  end, files)
  
  pickers.new(opts, {
    prompt_title = string.format("Files tagged #%s", tag),
    finder = finders.new_table {
      results = file_strings,
    },
    sorter = conf.file_sorter(opts),
    previewer = conf.file_previewer(opts),
    attach_mappings = function(prompt_bufnr, map)
      actions.select_default:replace(function()
        local selection = action_state.get_selected_entry()
        actions.close(prompt_bufnr)
        vim.cmd("edit " .. vim.fn.fnameescape(selection[1]))
      end)
      return true
    end,
  }):find()
end

return M

