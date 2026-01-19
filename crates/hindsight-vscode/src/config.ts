import * as vscode from 'vscode';

/**
 * Configuration helper functions for Hindsight MCP extension
 */

export interface HindsightConfig {
    autoStart: boolean;
    databasePath: string;
    binaryPath: string;
}

/**
 * Get the current Hindsight configuration
 */
export function getConfig(): HindsightConfig {
    const config = vscode.workspace.getConfiguration('hindsight');
    return {
        autoStart: config.get<boolean>('autoStart', true),
        databasePath: config.get<string>('databasePath', ''),
        binaryPath: config.get<string>('binaryPath', ''),
    };
}

/**
 * Get environment variables for the MCP server process
 */
export function getServerEnvironment(): Record<string, string> {
    const config = getConfig();
    const env: Record<string, string> = {};

    if (config.databasePath) {
        env['HINDSIGHT_DB'] = config.databasePath;
    }

    return env;
}

/**
 * Watch for configuration changes
 */
export function onConfigurationChange(
    callback: () => void
): vscode.Disposable {
    return vscode.workspace.onDidChangeConfiguration((e) => {
        if (e.affectsConfiguration('hindsight')) {
            callback();
        }
    });
}
