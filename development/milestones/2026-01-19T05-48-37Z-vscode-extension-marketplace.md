# MILESTONE: Publish hindsight-mcp to VS Code Extension Marketplace

**Status**: ✅ COMPLETE
**Priority**: 🟡 MEDIUM
**Created**: 2026-01-19T05:48:37Z
**Completed**: 2026-01-19
**Estimated Duration**: 2-3 sessions
**Prerequisites**: MCP Registry publication complete ✅

---

## Executive Summary

**Objective**: Create and publish a VS Code extension that provides hindsight-mcp as an integrated MCP server, available on the Visual Studio Code Marketplace.

**Current State**: 
- hindsight-mcp v0.1.5 is published on crates.io
- hindsight-mcp is published to MCP Registry (`io.github.Rbfinch/hindsight-mcp`)
- Pre-built binaries available via GitHub Releases for 5 platforms
- Users can already install via `@mcp` in VS Code Extensions view (from MCP Registry)

**The Opportunity**: 
While the MCP Registry integration provides basic installation, a dedicated VS Code extension offers:
1. **Richer Integration** - Settings, commands, activation events, status bar integration
2. **Automatic Binary Management** - Download and manage the correct binary for the user's platform
3. **Better Discoverability** - Listed in the main VS Code Marketplace alongside other extensions
4. **Enhanced UX** - Welcome walkthrough, output channel, error handling
5. **Contribution Points** - Provide MCP servers via `mcpServerDefinitionProviders` API

**The Solution**:
1. Create a VS Code extension that wraps hindsight-mcp
2. Use `mcpServerDefinitionProviders` contribution point
3. Implement automatic binary download and platform detection
4. Publish to Visual Studio Code Marketplace
5. This supplements (does not replace) MCP Registry and crates.io

---

## Success Criteria

| Metric | Target | Status |
|--------|--------|--------|
| Extension scaffolded | Working TypeScript extension | ✅ Complete |
| Binary management | Auto-download platform binary | ✅ Complete |
| MCP provider implemented | McpServerDefinitionProvider registered | ✅ Complete |
| Extension settings | Configurable workspace path, database location | ✅ Complete |
| Extension packaged | .vsix file created | ✅ Complete |
| Publisher account | Verified publisher on Marketplace | ✅ Complete |
| Published to Marketplace | Searchable and installable | ✅ Complete |
| MCP Registry unaffected | Still discoverable via @mcp | ✅ Verified |
| crates.io unaffected | `cargo install hindsight-mcp` works | ✅ Verified |

**Marketplace URL:** https://marketplace.visualstudio.com/items?itemName=Rbfinch.hindsight-mcp

---

## Background Research

### VS Code MCP Extension Architecture

VS Code supports two ways to use MCP servers:

1. **MCP Registry (via @MCP gallery)**
   - User discovers via Extensions view → @mcp search
   - VS Code adds entry to `mcp.json`
   - User downloads/runs binary manually or via registry config
   
2. **VS Code Extension (via Marketplace)**
   - Extension contributes `mcpServerDefinitionProviders`
   - Extension manages binary lifecycle
   - Richer integration with VS Code settings, commands, UI

### Key VS Code APIs

```typescript
// McpServerDefinitionProvider interface
interface McpServerDefinitionProvider<T extends McpServerDefinition> {
  readonly onDidChangeMcpServerDefinitions?: Event<void>
  provideMcpServerDefinitions(token: CancellationToken): ProviderResult<T[]>
  resolveMcpServerDefinition?(server: T, token: CancellationToken): ProviderResult<T>
}

// Register provider in extension activation
lm.registerMcpServerDefinitionProvider(id, provider)
```

### Package.json Contribution Point

```json
{
  "contributes": {
    "mcpServerDefinitionProviders": [
      {
        "id": "hindsight-mcp.servers",
        "label": "Hindsight MCP"
      }
    ]
  }
}
```

### Binary Distribution Strategy

| Option | Pros | Cons |
|--------|------|------|
| Bundle in extension | No download needed | Large extension size (5 binaries × 3-5MB) |
| Download on activation | Small extension | Network dependency, first-run delay |
| GitHub Release download | Existing infrastructure | Requires version coordination |
| NPM binary wrapper | Familiar pattern | Another package to maintain |

**Recommended**: Download from GitHub Releases on first activation, with progress indication.

---

## Phase Breakdown

### Phase 0: Extension Scaffolding (0.5 session)

**Status**: ✅ COMPLETE
**Goal**: Create the VS Code extension project structure

#### Tasks

1. **Create extension directory** (~5 min)
   ```bash
   mkdir -p crates/hindsight-vscode
   cd crates/hindsight-vscode
   ```

2. **Initialize extension with Yeoman** (~10 min)
   ```bash
   npx --yes yo generator-code
   # Select: New Extension (TypeScript)
   # Name: hindsight-mcp
   # Identifier: hindsight-mcp
   # Description: MCP server for AI-assisted coding with development history
   # Initialize git: No (already in monorepo)
   # Bundle with webpack: Yes
   # Package manager: npm
   ```

3. **Configure package.json** (~15 min)
   Update `package.json` with:
   ```json
   {
     "name": "hindsight-mcp",
     "displayName": "Hindsight MCP",
     "description": "MCP server for AI-assisted coding with development history: git, tests, Copilot.",
     "version": "0.1.5",
     "publisher": "Rbfinch",
     "engines": {
       "vscode": "^1.102.0"
     },
     "categories": ["AI", "Other"],
     "keywords": ["mcp", "ai", "copilot", "git", "development-history"],
     "activationEvents": [],
     "main": "./dist/extension.js",
     "contributes": {
       "mcpServerDefinitionProviders": [
         {
           "id": "hindsight-mcp.servers",
           "label": "Hindsight MCP"
         }
       ],
       "configuration": {
         "title": "Hindsight MCP",
         "properties": {
           "hindsight.autoStart": {
             "type": "boolean",
             "default": true,
             "description": "Automatically start hindsight-mcp server"
           },
           "hindsight.databasePath": {
             "type": "string",
             "default": "",
             "description": "Custom path for the hindsight database (default: ~/.hindsight/hindsight.db)"
           }
         }
       },
       "commands": [
         {
           "command": "hindsight.downloadBinary",
           "title": "Download Hindsight Binary",
           "category": "Hindsight"
         },
         {
           "command": "hindsight.showVersion",
           "title": "Show Hindsight Version",
           "category": "Hindsight"
         }
       ]
     },
     "repository": {
       "type": "git",
       "url": "https://github.com/Rbfinch/hindsight-mcp"
     },
     "license": "MIT"
   }
   ```

4. **Set up project structure** (~10 min)
   ```
   crates/hindsight-vscode/
   ├── .vscode/
   │   ├── launch.json
   │   └── tasks.json
   ├── src/
   │   ├── extension.ts       # Main activation
   │   ├── binaryManager.ts   # Download/install binary
   │   ├── mcpProvider.ts     # McpServerDefinitionProvider
   │   └── config.ts          # Configuration helpers
   ├── package.json
   ├── tsconfig.json
   ├── webpack.config.js
   └── README.md
   ```

#### Validation Gate

```bash
# Extension compiles
npm run compile

# Extension can be packaged (dry run)
npx vsce package --dry-run
```

---

### Phase 1: Binary Management (1 session)

**Status**: ✅ COMPLETE
**Goal**: Implement automatic binary download and management
**Dependencies**: Phase 0 complete

#### Tasks

1. **Implement platform detection** (~15 min)
   ```typescript
   // src/binaryManager.ts
   interface PlatformInfo {
     os: 'linux' | 'darwin' | 'windows';
     arch: 'x64' | 'arm64';
     binaryName: string;
     archiveName: string;
   }
   
   function getPlatformInfo(): PlatformInfo {
     const os = process.platform;
     const arch = process.arch;
     // Map to GitHub Release asset names
   }
   ```

2. **Implement binary download** (~30 min)
   ```typescript
   async function downloadBinary(
     version: string,
     progress: Progress<{ message?: string; increment?: number }>
   ): Promise<string> {
     // 1. Check if binary already exists
     // 2. Download from GitHub Releases
     // 3. Extract archive (tar.gz or zip)
     // 4. Set executable permissions
     // 5. Return binary path
   }
   ```

3. **Implement version checking** (~15 min)
   ```typescript
   async function checkForUpdates(): Promise<boolean> {
     // Compare installed version with latest GitHub Release
   }
   
   async function getInstalledVersion(): Promise<string | null> {
     // Run binary --version and parse output
   }
   ```

4. **Handle binary storage** (~15 min)
   ```typescript
   function getBinaryStoragePath(context: ExtensionContext): string {
     // Use context.globalStorageUri for cross-platform storage
     // ~/.vscode/extensions/hindsight-mcp/bin/
   }
   ```

5. **Implement extraction** (~20 min)
   - Handle `.tar.gz` for Linux/macOS
   - Handle `.zip` for Windows
   - Set executable permissions on Unix

#### Validation Gate

```typescript
// Test binary download on each platform
const binaryPath = await downloadBinary('0.1.5', progress);
const version = await getInstalledVersion();
assert.equal(version, '0.1.5');
```

---

### Phase 2: MCP Provider Implementation (0.5 session)

**Status**: ✅ COMPLETE
**Goal**: Implement McpServerDefinitionProvider
**Dependencies**: Phase 1 complete

#### Tasks

1. **Implement McpServerDefinitionProvider** (~20 min)
   ```typescript
   // src/mcpProvider.ts
   import * as vscode from 'vscode';
   
   export class HindsightMcpProvider implements vscode.McpServerDefinitionProvider<vscode.McpStdioServerDefinition> {
     private binaryPath: string | undefined;
     private _onDidChange = new vscode.EventEmitter<void>();
     
     readonly onDidChangeMcpServerDefinitions = this._onDidChange.event;
     
     constructor(private context: vscode.ExtensionContext) {}
     
     async provideMcpServerDefinitions(
       token: vscode.CancellationToken
     ): Promise<vscode.McpStdioServerDefinition[]> {
       if (!this.binaryPath) {
         return [];
       }
       
       return [{
         name: 'hindsight',
         displayName: 'Hindsight MCP',
         command: this.binaryPath,
         args: ['serve'],
         env: this.getEnvironment()
       }];
     }
     
     async resolveMcpServerDefinition(
       server: vscode.McpStdioServerDefinition,
       token: vscode.CancellationToken
     ): Promise<vscode.McpStdioServerDefinition> {
       // Ensure binary is downloaded
       if (!this.binaryPath) {
         await this.ensureBinary();
       }
       return server;
     }
     
     private getEnvironment(): Record<string, string> {
       const config = vscode.workspace.getConfiguration('hindsight');
       const dbPath = config.get<string>('databasePath');
       return dbPath ? { HINDSIGHT_DB: dbPath } : {};
     }
   }
   ```

2. **Register provider in activation** (~10 min)
   ```typescript
   // src/extension.ts
   export async function activate(context: vscode.ExtensionContext) {
     const provider = new HindsightMcpProvider(context);
     
     // Ensure binary exists on activation
     await provider.ensureBinary();
     
     // Register the MCP server definition provider
     const disposable = vscode.lm.registerMcpServerDefinitionProvider(
       'hindsight-mcp.servers',
       provider
     );
     
     context.subscriptions.push(disposable);
   }
   ```

3. **Handle configuration changes** (~10 min)
   ```typescript
   // Watch for configuration changes
   vscode.workspace.onDidChangeConfiguration(e => {
     if (e.affectsConfiguration('hindsight')) {
       provider.notifyChange();
     }
   });
   ```

#### Validation Gate

```bash
# Test in VS Code Extension Host
# 1. Press F5 to launch extension development host
# 2. Open Command Palette → MCP: List Servers
# 3. Verify "Hindsight MCP" appears in list
# 4. Start the server and verify tools are available
```

---

### Phase 3: Polish and Testing (0.5 session)

**Status**: ✅ COMPLETE
**Goal**: Add polish, error handling, and comprehensive testing
**Dependencies**: Phase 2 complete

#### Tasks

1. **Add output channel for logging** (~15 min)
   ```typescript
   const outputChannel = vscode.window.createOutputChannel('Hindsight MCP');
   outputChannel.appendLine(`[INFO] Hindsight MCP activated`);
   ```

2. **Add progress indication for binary download** (~10 min)
   ```typescript
   await vscode.window.withProgress({
     location: vscode.ProgressLocation.Notification,
     title: 'Downloading Hindsight MCP...',
     cancellable: false
   }, async (progress) => {
     await downloadBinary(version, progress);
   });
   ```

3. **Add error handling** (~15 min)
   - Handle network failures gracefully
   - Show user-friendly error messages
   - Provide manual download fallback

4. **Create README.md for extension** (~15 min)
   - Feature overview
   - Installation instructions
   - Configuration options
   - Troubleshooting guide

5. **Add extension icon** (~10 min)
   - Create 128x128 PNG icon
   - Add to package.json: `"icon": "images/icon.png"`

6. **Write integration tests** (~20 min)
   ```typescript
   // src/test/suite/extension.test.ts
   test('Extension activates', async () => {
     const ext = vscode.extensions.getExtension('Rbfinch.hindsight-mcp');
     await ext?.activate();
     assert.ok(ext?.isActive);
   });
   
   test('Binary downloads successfully', async () => {
     // Test binary download on current platform
   });
   ```

#### Validation Gate

```bash
# Run tests
npm test

# Package extension
npx vsce package

# Verify .vsix file size is reasonable (<10MB)
ls -la hindsight-mcp-*.vsix
```

---

### Phase 4: Marketplace Publication (0.5 session)

**Status**: ✅ COMPLETE
**Goal**: Publish extension to VS Code Marketplace
**Dependencies**: Phase 3 complete

#### Tasks

1. **Create publisher account** (~10 min) ✅
   - Go to https://marketplace.visualstudio.com/manage
   - Create publisher "Rbfinch" (or use existing)
   - Verify publisher with Personal Access Token

2. **Generate Personal Access Token** (~5 min) ✅
   - Go to Azure DevOps: https://dev.azure.com
   - Create PAT with Marketplace (Manage) scope
   - Store securely

3. **Login to vsce** (~5 min)
   ```bash
   npx vsce login Rbfinch
   # Enter PAT when prompted
   ```

4. **Pre-publish checklist** (~10 min)
   - [ ] Version matches hindsight-mcp crate version
   - [ ] README.md is complete and accurate
   - [ ] Icon is included (128x128 PNG)
   - [ ] LICENSE file is present
   - [ ] CHANGELOG.md exists
   - [ ] No sensitive data in package

5. **Publish to Marketplace** (~5 min)
   ```bash
   npx vsce publish
   # Or publish specific version
   npx vsce publish 0.1.5
   ```

6. **Verify publication** (~5 min)
   - Visit https://marketplace.visualstudio.com/items?itemName=Rbfinch.hindsight-mcp
   - Verify extension details are correct
   - Test installation in VS Code

#### Deliverables

- Published extension on VS Code Marketplace
- `hindsight-mcp-0.1.5.vsix` in releases

#### Validation Gate

```bash
# Verify extension is installable
code --install-extension Rbfinch.hindsight-mcp

# Verify it appears in Extensions view
# Search for "hindsight" in Extensions
```

---

## Future Considerations

### Continuous Deployment

Once manual publishing is validated, automate with GitHub Actions:

```yaml
# .github/workflows/extension.yml
name: VS Code Extension

on:
  push:
    tags:
      - 'v*'

jobs:
  publish:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Setup Node.js
        uses: actions/setup-node@v4
        with:
          node-version: '20'
      
      - name: Install dependencies
        run: |
          cd crates/hindsight-vscode
          npm ci
      
      - name: Build extension
        run: |
          cd crates/hindsight-vscode
          npm run compile
      
      - name: Publish to Marketplace
        run: |
          cd crates/hindsight-vscode
          npx vsce publish
        env:
          VSCE_PAT: ${{ secrets.VSCE_PAT }}
```

### Feature Roadmap

| Feature | Priority | Description |
|---------|----------|-------------|
| Status bar indicator | Medium | Show server status in status bar |
| Auto-update binary | Medium | Check for updates on activation |
| Workspace detection | Low | Auto-configure for workspace projects |
| Telemetry (opt-in) | Low | Usage analytics for improvement |
| Welcome walkthrough | Low | Guided setup for new users |

---

## Risks and Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Binary download fails | High | Provide manual download fallback, clear error messages |
| Platform not supported | Medium | Error message with alternative (cargo install) |
| Version mismatch | Medium | Coordinate extension and binary versions |
| Publisher verification issues | Low | Create Azure DevOps org in advance |
| Large extension size | Low | Download binary on demand, not bundled |

---

## References

- [VS Code Extension API](https://code.visualstudio.com/api)
- [McpServerDefinitionProvider API](https://code.visualstudio.com/api/references/vscode-api#McpServerDefinitionProvider)
- [Publishing Extensions](https://code.visualstudio.com/api/working-with-extensions/publishing-extension)
- [Extension Manifest](https://code.visualstudio.com/api/references/extension-manifest)
- [MCP Servers in VS Code](https://code.visualstudio.com/docs/copilot/chat/mcp-servers)
- [vsce CLI](https://github.com/microsoft/vscode-vsce)

---

## Appendix: Extension Manifest Fields

| Field | Required | Description |
|-------|----------|-------------|
| `name` | Yes | Extension identifier (lowercase, no spaces) |
| `displayName` | Yes | Human-readable name |
| `description` | Yes | Brief description (max 200 chars) |
| `version` | Yes | Semver version |
| `publisher` | Yes | Publisher ID |
| `engines.vscode` | Yes | Minimum VS Code version |
| `categories` | No | Marketplace categories |
| `keywords` | No | Search keywords (max 5) |
| `icon` | No | 128x128 PNG icon |
| `repository` | No | Source repository URL |
| `contributes` | No | Contribution points |

---

## Appendix: Platform Binary Mapping

| Platform | Architecture | GitHub Release Asset |
|----------|--------------|---------------------|
| Linux | x64 | `hindsight-mcp-vX.Y.Z-x86_64-unknown-linux-gnu.tar.gz` |
| Linux | ARM64 | `hindsight-mcp-vX.Y.Z-aarch64-unknown-linux-gnu.tar.gz` |
| macOS | Intel | `hindsight-mcp-vX.Y.Z-x86_64-apple-darwin.tar.gz` |
| macOS | Apple Silicon | `hindsight-mcp-vX.Y.Z-aarch64-apple-darwin.tar.gz` |
| Windows | x64 | `hindsight-mcp-vX.Y.Z-x86_64-pc-windows-msvc.zip` |
