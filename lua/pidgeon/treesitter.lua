local M = {} 

local EVALUABLE_NODES = {
  'function_call',
  'binary_expression',
  'unary_expression',
  'table_constructor',
  'function_definition',
  'parenthesized_expression',

  -- statements
  'variable_declaration',
  'assignment_statement',
  'return_statement',
  'for_statement',
  'while_statement',
  'if_statement',

  -- top-level constructs
  'function_declaration',
  'local_function',
}

local function isEvaluable(node)
  local nodeType = node:type()
  for _, evaluableType in ipairs(EVALUABLE_NODES) do 
    if nodeType == evaluableType then return true end
  end

  return false
end

local function getNodeText(node, bufnr) 
  local sr, sc, er, ec = node:range()
  local lines = vim.api.nvim_buf_get_lines(bufnr, sr, er + 1, false)

  if #lines == 0 then 
    return nil
  end

  if #lines == 1 then 
    return lines[1]:sub(sc + 1, ec)
  end

  lines[1] = lines[1]:sub(sc + 1, ec)
  lines[#lines] = lines[#lines]:sub(1, ec)

  return table.concat(lines, '\n')
end

local function getNearest(bufnr)
  bufnr = bufnr or vim.api.nvim_get_current_buf()

  local ok, parser = pcall(vim.treesitter.get_parser, bufnr, 'lua')
  if not ok then 
    vim.notify('treesitter lua not installed', vim.log.levels.ERROR)
    return nil
  end

  local cursor = vim.api.nvim_win_get_cursor(0)
  local row, col = cursor[1] - 1, cursor[2]

  local tree = parser:parse()[1]
  local root = tree:root()

  local node = root:named_descendant_for_range(row, col, row, col)

  if not node then 
    vim.notify('no node at cursor', vim.log.levels.WARN)
    return nil
  end

  while node do 
    if isEvaluable(node) then 
      local text = getNodeText(node, bufnr)
      local sr, sc, er, ec = node:range()

      return {
        text = text,
        node = node,
        range = {
          startRow = sr,
          startCol = sc,
          endRow = er,
          endCol = ec,
        }
      }
    end

  node = node:parent()
  end

  vim.notify('expression not evaluable', vim.log.levels.WARN)
  return nil
end

function M.sendNearest() 
  local expr = getNearest()

  if not expr then 
    return
  end

  local ns = vim.api.nvim_create_namespace('pidgeon_flash')
  vim.highlight.range(
    0, 
    ns, 
    'Visual', 
    {expr.range.startRow, expr.range.startCol}, 
    {expr.range.endRow, expr.range.endCol}, 
    {}
  )

  require('pidgeon.client').send(expr.text)

  vim.defer_fn(function()
    vim.api.nvim_buf_clear_namespace(0, ns, 0, -1)
  end, 500)
end

return M
