# Hindsight MCP

**MCP server for AI-assisted coding with development history: git, tests, Copilot.**

Hindsight MCP gives AI agents the ability to explore your development history, including git commits, test runs, and Copilot sessions. This context helps AI provide more accurate and relevant assistance.

## Features

- **Git History** - Search commits, view diffs, and explore your repository timeline
- **Test Results** - Find failing tests, track test history by commit
- **Copilot Sessions** - Review AI-assisted coding sessions
- **Activity Summary** - Get aggregate statistics about development activity

## Installation

### From VS Code Marketplace

1. Open VS Code
2. Go to Extensions (Ctrl+Shift+X / Cmd+Shift+X)
3. Search for "Hindsight MCP"
4. Click Install

### From Command Line

```bash
code --install-extension Rbfinch.hindsight-mcp
```

## Usage

Once installed, the extension automatically provides Hindsight MCP as an MCP server to VS Code's AI features.

### Available Tools

The MCP server provides these tools to AI agents:

| Tool | Description |
|------|-------------|
| `hindsight_timeline` | View chronological development activity |
| `hindsight_search` | Full-text search across commits and messages |
| `hindsight_failing_tests` | Get currently failing tests |
| `hindsight_activity_summary` | Aggregate activity statistics |
| `hindsight_commit_details` | Detailed commit information |
| `hindsight_ingest` | Trigger data ingestion from sources |

### First-Time Setup

1. **Ingest your data**: The first time you use Hindsight, ingest your git history:
   - In Copilot Chat, try: "Ingest my git history with hindsight"

2. **Explore your history**: Ask AI about your development activity:
   - "What commits did I make this week?"
   - "Show me failing tests"
   - "Summarize my development activity for the last 7 days"

## Configuration

Configure Hindsight MCP through VS Code settings:

| Setting | Default | Description |
|---------|---------|-------------|
| `hindsight.autoStart` | `true` | Automatically start hindsight-mcp server |
| `hindsight.databasePath` | `""` | Custom path for the hindsight database |
| `hindsight.binaryPath` | `""` | Custom path to hindsight-mcp binary |

### Example Settings

```json
{
  "hindsight.autoStart": true,
  "hindsight.databasePath": "~/.hindsight/my-project.db"
}
```

## Commands

Open the Command Palette (Ctrl+Shift+P / Cmd+Shift+P) and search for:

| Command | Description |
|---------|-------------|
| `Hindsight: Download Binary` | Manually download the hindsight-mcp binary |
| `Hindsight: Show Version` | Display the installed version |

## Alternative Installation Methods

If you prefer not to use the extension's binary management:

### cargo install

```bash
cargo install hindsight-mcp
```

Then set the custom binary path in settings:

```json
{
  "hindsight.binaryPath": "/path/to/hindsight-mcp"
}
```

### MCP Registry

Hindsight MCP is also available via the MCP Registry:
- Search for `@mcp hindsight` in VS Code Extensions

## Requirements

- VS Code 1.102.0 or later
- Internet connection for binary download (first-time only)

## Supported Platforms

| Platform | Architecture | Support |
|----------|--------------|---------|
| Linux | x64 | ✅ |
| Linux | ARM64 | ✅ |
| macOS | Intel (x64) | ✅ |
| macOS | Apple Silicon (ARM64) | ✅ |
| Windows | x64 | ✅ |

## Troubleshooting

### Binary download fails

If automatic download fails:
1. Check your internet connection
2. Try manual download via Command Palette: "Hindsight: Download Binary"
3. Install via cargo: `cargo install hindsight-mcp`

### Server not appearing in MCP list

1. Check Output panel (View → Output → select "Hindsight MCP")
2. Ensure `hindsight.autoStart` is `true`
3. Reload VS Code window

### Custom binary not found

Ensure the path in `hindsight.binaryPath` is absolute and the file is executable.

## Links

- [GitHub Repository](https://github.com/Rbfinch/hindsight-mcp)
- [Issues](https://github.com/Rbfinch/hindsight-mcp/issues)
- [Changelog](https://github.com/Rbfinch/hindsight-mcp/blob/main/CHANGELOG.md)
- [crates.io](https://crates.io/crates/hindsight-mcp)

## License

MIT License - see [LICENSE](https://github.com/Rbfinch/hindsight-mcp/blob/main/LICENSE.md) for details.
