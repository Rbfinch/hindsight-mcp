import * as vscode from 'vscode';
import { HindsightMcpProvider } from './mcpProvider';
import {
    downloadBinary,
    getInstalledVersion,
    getExpectedVersion,
    binaryExists,
} from './binaryManager';

let outputChannel: vscode.OutputChannel;
let provider: HindsightMcpProvider;

/**
 * Activate the Hindsight MCP extension
 */
export async function activate(context: vscode.ExtensionContext): Promise<void> {
    // Create output channel for logging
    outputChannel = vscode.window.createOutputChannel('Hindsight MCP');
    context.subscriptions.push(outputChannel);

    outputChannel.appendLine(`[INFO] Hindsight MCP extension activating...`);
    outputChannel.appendLine(`[INFO] Expected version: ${getExpectedVersion()}`);

    // Create the MCP provider
    provider = new HindsightMcpProvider(context, outputChannel);
    context.subscriptions.push(provider);

    // Initialize the provider (check for binary)
    await provider.initialize();

    // Register the MCP server definition provider
    const disposable = vscode.lm.registerMcpServerDefinitionProvider(
        'hindsight-mcp.servers',
        provider
    );
    context.subscriptions.push(disposable);

    outputChannel.appendLine('[INFO] MCP server definition provider registered');

    // Register commands
    registerCommands(context);

    outputChannel.appendLine('[INFO] Hindsight MCP extension activated');
}

/**
 * Register extension commands
 */
function registerCommands(context: vscode.ExtensionContext): void {
    // Download Binary command
    const downloadCmd = vscode.commands.registerCommand(
        'hindsight.downloadBinary',
        async () => {
            try {
                await vscode.window.withProgress(
                    {
                        location: vscode.ProgressLocation.Notification,
                        title: `Downloading Hindsight MCP v${getExpectedVersion()}...`,
                        cancellable: false,
                    },
                    async (progress) => {
                        await downloadBinary(context, progress);
                    }
                );

                vscode.window.showInformationMessage(
                    `Hindsight MCP v${getExpectedVersion()} downloaded successfully!`
                );

                // Notify provider that binary is now available
                provider.notifyChange();
            } catch (error) {
                const message = error instanceof Error ? error.message : String(error);
                vscode.window.showErrorMessage(`Failed to download Hindsight MCP: ${message}`);
            }
        }
    );
    context.subscriptions.push(downloadCmd);

    // Show Version command
    const versionCmd = vscode.commands.registerCommand(
        'hindsight.showVersion',
        async () => {
            if (!binaryExists(context)) {
                vscode.window.showWarningMessage(
                    'Hindsight MCP binary not installed. Run "Hindsight: Download Binary" first.'
                );
                return;
            }

            const installedVersion = await getInstalledVersion(context);
            const expectedVersion = getExpectedVersion();

            if (installedVersion) {
                const message =
                    installedVersion === expectedVersion
                        ? `Hindsight MCP v${installedVersion} (up to date)`
                        : `Hindsight MCP v${installedVersion} (expected: v${expectedVersion})`;
                vscode.window.showInformationMessage(message);
            } else {
                vscode.window.showWarningMessage('Could not determine Hindsight MCP version');
            }
        }
    );
    context.subscriptions.push(versionCmd);
}

/**
 * Deactivate the extension
 */
export function deactivate(): void {
    if (outputChannel) {
        outputChannel.appendLine('[INFO] Hindsight MCP extension deactivating...');
    }
}
