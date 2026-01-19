import * as vscode from 'vscode';
import { getConfig, getServerEnvironment, onConfigurationChange } from './config';
import {
    getBinaryPath,
    binaryExists,
    downloadBinary,
    getExpectedVersion,
} from './binaryManager';

/**
 * Hindsight MCP Server Definition Provider
 * Provides hindsight-mcp as an MCP server to VS Code
 */
export class HindsightMcpProvider
    implements vscode.McpServerDefinitionProvider<vscode.McpStdioServerDefinition> {
    private binaryPath: string | null = null;
    private readonly _onDidChange = new vscode.EventEmitter<void>();
    private readonly outputChannel: vscode.OutputChannel;

    readonly onDidChangeMcpServerDefinitions = this._onDidChange.event;

    constructor(
        private readonly context: vscode.ExtensionContext,
        outputChannel: vscode.OutputChannel
    ) {
        this.outputChannel = outputChannel;

        // Watch for configuration changes
        context.subscriptions.push(
            onConfigurationChange(() => {
                this.outputChannel.appendLine('[INFO] Configuration changed, notifying MCP server change');
                this._onDidChange.fire();
            })
        );
    }

    /**
     * Initialize the provider - check/download binary
     */
    async initialize(): Promise<void> {
        const config = getConfig();

        // Check for custom binary path first
        if (config.binaryPath) {
            this.binaryPath = config.binaryPath;
            this.outputChannel.appendLine(`[INFO] Using custom binary path: ${this.binaryPath}`);
            return;
        }

        // Check if binary exists
        if (binaryExists(this.context)) {
            this.binaryPath = getBinaryPath(this.context);
            this.outputChannel.appendLine(`[INFO] Binary found at: ${this.binaryPath}`);
            return;
        }

        // Binary doesn't exist, need to download
        this.outputChannel.appendLine('[INFO] Binary not found, download required');
        this.binaryPath = null;
    }

    /**
     * Ensure binary is available, downloading if necessary
     */
    async ensureBinary(): Promise<string | null> {
        const config = getConfig();

        // Custom binary path takes precedence
        if (config.binaryPath) {
            this.binaryPath = config.binaryPath;
            return this.binaryPath;
        }

        // Check if already downloaded
        if (this.binaryPath && binaryExists(this.context)) {
            return this.binaryPath;
        }

        // Download with progress
        try {
            this.binaryPath = await vscode.window.withProgress(
                {
                    location: vscode.ProgressLocation.Notification,
                    title: `Downloading Hindsight MCP v${getExpectedVersion()}...`,
                    cancellable: false,
                },
                async (progress) => {
                    return await downloadBinary(this.context, progress);
                }
            );

            this.outputChannel.appendLine(`[INFO] Binary downloaded to: ${this.binaryPath}`);
            this._onDidChange.fire();
            return this.binaryPath;
        } catch (error) {
            const message = error instanceof Error ? error.message : String(error);
            this.outputChannel.appendLine(`[ERROR] Failed to download binary: ${message}`);
            vscode.window.showErrorMessage(
                `Failed to download Hindsight MCP: ${message}. You can install manually with: cargo install hindsight-mcp`
            );
            return null;
        }
    }

    /**
     * Provide MCP server definitions
     */
    async provideMcpServerDefinitions(
        _token: vscode.CancellationToken
    ): Promise<vscode.McpStdioServerDefinition[]> {
        // Ensure binary is available
        if (!this.binaryPath) {
            await this.ensureBinary();
        }

        if (!this.binaryPath) {
            this.outputChannel.appendLine('[WARN] No binary available, returning empty server list');
            return [];
        }

        const config = getConfig();
        if (!config.autoStart) {
            this.outputChannel.appendLine('[INFO] Auto-start disabled, returning empty server list');
            return [];
        }

        // McpStdioServerDefinition constructor: (label, command, args?, env?, version?)
        const serverDef = new vscode.McpStdioServerDefinition(
            'Hindsight MCP',
            this.binaryPath,
            ['serve'],
            getServerEnvironment(),
            getExpectedVersion()
        );

        this.outputChannel.appendLine('[INFO] Providing Hindsight MCP server definition');
        return [serverDef];
    }

    /**
     * Resolve an MCP server definition (ensure binary is ready)
     */
    async resolveMcpServerDefinition(
        server: vscode.McpStdioServerDefinition,
        _token: vscode.CancellationToken
    ): Promise<vscode.McpStdioServerDefinition> {
        // Ensure binary is available before starting
        if (!this.binaryPath) {
            await this.ensureBinary();
        }

        // Return a new server definition with the correct binary path
        if (this.binaryPath) {
            return new vscode.McpStdioServerDefinition(
                'Hindsight MCP',
                this.binaryPath,
                ['serve'],
                getServerEnvironment(),
                getExpectedVersion()
            );
        }

        return server;
    }

    /**
     * Notify that server definitions have changed
     */
    notifyChange(): void {
        this._onDidChange.fire();
    }

    /**
     * Get the current binary path
     */
    getBinaryPath(): string | null {
        return this.binaryPath;
    }

    dispose(): void {
        this._onDidChange.dispose();
    }
}
