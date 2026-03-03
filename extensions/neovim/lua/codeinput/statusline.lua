local M = {}

local function get_initials(identifier)
  if identifier:sub(1, 1) == "@" then
    local parts = {}
    for part in identifier:gmatch("[^/]+") do
      table.insert(parts, part)
    end
    
    if #parts > 1 then
      return parts[#parts]:sub(1, 1):upper()
    else
      return identifier:sub(2, 2):upper()
    end
  end
  
  if identifier:find("@") then
    return identifier:sub(1, 1):upper()
  end
  
  return identifier:sub(1, 1):upper()
end

local function get_cached_ownership()
  local bufnr = vim.api.nvim_get_current_buf()
  local cached = vim.b[bufnr].codeinput_ownership
  if cached and cached.timestamp and (os.time() - cached.timestamp) < 5 then
    return cached.data
  end
  return nil
end

local function set_cached_ownership(data)
  local bufnr = vim.api.nvim_get_current_buf()
  vim.b[bufnr].codeinput_ownership = {
    data = data,
    timestamp = os.time()
  }
end

function M.get_status()
  local clients = vim.lsp.get_active_clients({ name = "codeinput" })
  if #clients == 0 then
    return ""
  end
  
  local cached = get_cached_ownership()
  if cached then
    return cached.status or ""
  end
  
  local params = vim.lsp.util.make_position_params(0, "utf-16")
  
  vim.lsp.buf_request(0, "textDocument/hover", params, function(err, result, ctx)
    if err or not result or not result.contents then
      set_cached_ownership({ status = "" })
      return
    end
    
    local status = ""
    local is_unowned = false
    local owners = {}
    local tags = {}
    
    local contents = result.contents
    if type(contents) == "table" then
      if contents.kind == "markdown" then
        contents = { contents }
      end
      
      for _, item in ipairs(contents) do
        local text = ""
        if type(item) == "string" then
          text = item
        elseif item.value then
          text = item.value
        end
        
        if text:match("Owners:%s*%(none%)") then
          is_unowned = true
        elseif text:match("Owners:") then
          local owner_str = text:match("%*%*Owners:%*%*%s*(.+)")
          if owner_str and owner_str ~= "(none)" then
            for owner in owner_str:gmatch("`([^`]+)`") do
              table.insert(owners, owner)
            end
          end
        end
        
        if text:match("Tags:") then
          local tag_str = text:match("%*%*Tags:%*%*%s*(.+)")
          if tag_str then
            for tag in tag_str:gmatch("`#([^`]+)`") do
              table.insert(tags, tag)
            end
          end
        end
        
        if text:match("Warning") then
          is_unowned = true
        end
      end
    end
    
    if is_unowned or #owners == 0 then
      status = ""
    else
      local initials = ""
      for _, owner in ipairs(owners) do
        initials = initials .. get_initials(owner)
      end
      status = " " .. initials
    end
    
    set_cached_ownership({ status = status })
  end)
  
  local cached_now = get_cached_ownership()
  return cached_now and cached_now.status or ""
end

return M
