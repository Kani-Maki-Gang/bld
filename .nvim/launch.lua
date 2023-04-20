return {
  rust = {
    {
      name = 'Launch',
      type = 'lldb',
      request = 'launch',
      program = function()
        return vim.fn.input('Path to executable: ', vim.fn.getcwd() .. '/', 'file')
      end,
      cwd = '${workspaceFolder}',
      stopOnEntry = false,
      args = {'check', '-p', 'sample.yaml'},
      prelaunchTask = "cargo build"
    },
    {
      name = 'Run pipeline',
      type = 'lldb',
      request = 'launch',
      program = function()
        return vim.fn.getcwd() .. '/target/debug/bld'
      end,
      cwd = '${workspaceFolder}',
      stopOnEntry = false,
      args = { 'run', '-p', 'build-musl.yaml' },
      prelaunchTask = function()
        print("Building project...")
        return "cargo build"
      end
    },
    {
      name = 'Run pipeline on server',
      type = 'lldb',
      request = 'launch',
      program = function()
        return vim.fn.getcwd() .. '/target/debug/bld'
      end,
      cwd = '${workspaceFolder}',
      stopOnEntry = false,
      args = { 'run', '-s', 'local', '-p', 'sample.yaml' },
      prelaunchTask = "cargo build"
    }
  }
}
