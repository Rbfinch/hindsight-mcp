import * as vscode from 'vscode';
import * as path from 'path';
import * as fs from 'fs';
import * as https from 'https';
import { exec } from 'child_process';

// Version of hindsight-mcp to download
const HINDSIGHT_VERSION = '0.1.5';
const GITHUB_REPO = 'Rbfinch/hindsight-mcp';

export interface PlatformInfo {
    os: 'linux' | 'darwin' | 'win32';
    arch: 'x64' | 'arm64';
    binaryName: string;
    archiveName: string;
    archiveExtension: 'tar.gz' | 'zip';
}

/**
 * Get platform-specific information for binary download
 */
export function getPlatformInfo(): PlatformInfo | null {
    const platform = process.platform;
    const arch = process.arch;

    // Map Node.js platform/arch to Rust target triples
    const targetMap: Record<string, Record<string, { target: string; ext: 'tar.gz' | 'zip' }>> = {
        linux: {
            x64: { target: 'x86_64-unknown-linux-gnu', ext: 'tar.gz' },
            arm64: { target: 'aarch64-unknown-linux-gnu', ext: 'tar.gz' },
        },
        darwin: {
            x64: { target: 'x86_64-apple-darwin', ext: 'tar.gz' },
            arm64: { target: 'aarch64-apple-darwin', ext: 'tar.gz' },
        },
        win32: {
            x64: { target: 'x86_64-pc-windows-msvc', ext: 'zip' },
        },
    };

    const platformTargets = targetMap[platform];
    if (!platformTargets) {
        return null;
    }

    const archTarget = platformTargets[arch];
    if (!archTarget) {
        return null;
    }

    const binaryName = platform === 'win32' ? 'hindsight-mcp.exe' : 'hindsight-mcp';
    const archiveName = `hindsight-mcp-v${HINDSIGHT_VERSION}-${archTarget.target}.${archTarget.ext}`;

    return {
        os: platform as 'linux' | 'darwin' | 'win32',
        arch: arch as 'x64' | 'arm64',
        binaryName,
        archiveName,
        archiveExtension: archTarget.ext,
    };
}

/**
 * Get the storage path for binaries
 */
export function getBinaryStoragePath(context: vscode.ExtensionContext): string {
    return path.join(context.globalStorageUri.fsPath, 'bin');
}

/**
 * Get the full path to the binary
 */
export function getBinaryPath(context: vscode.ExtensionContext): string | null {
    const platformInfo = getPlatformInfo();
    if (!platformInfo) {
        return null;
    }

    return path.join(getBinaryStoragePath(context), platformInfo.binaryName);
}

/**
 * Check if the binary exists
 */
export function binaryExists(context: vscode.ExtensionContext): boolean {
    const binaryPath = getBinaryPath(context);
    if (!binaryPath) {
        return false;
    }

    return fs.existsSync(binaryPath);
}

/**
 * Get the installed version of hindsight-mcp
 */
export async function getInstalledVersion(context: vscode.ExtensionContext): Promise<string | null> {
    const binaryPath = getBinaryPath(context);
    if (!binaryPath || !fs.existsSync(binaryPath)) {
        return null;
    }

    return new Promise((resolve) => {
        exec(`"${binaryPath}" --version`, (error: Error | null, stdout: string) => {
            if (error) {
                resolve(null);
                return;
            }
            // Parse "hindsight-mcp X.Y.Z" format
            const match = stdout.trim().match(/hindsight-mcp\s+(\d+\.\d+\.\d+)/);
            resolve(match ? match[1] : null);
        });
    });
}

/**
 * Download the binary for the current platform
 */
export async function downloadBinary(
    context: vscode.ExtensionContext,
    progress: vscode.Progress<{ message?: string; increment?: number }>
): Promise<string> {
    const platformInfo = getPlatformInfo();
    if (!platformInfo) {
        throw new Error(`Unsupported platform: ${process.platform} ${process.arch}`);
    }

    const storagePath = getBinaryStoragePath(context);

    // Create storage directory if it doesn't exist
    if (!fs.existsSync(storagePath)) {
        fs.mkdirSync(storagePath, { recursive: true });
    }

    const downloadUrl = `https://github.com/${GITHUB_REPO}/releases/download/v${HINDSIGHT_VERSION}/${platformInfo.archiveName}`;
    const archivePath = path.join(storagePath, platformInfo.archiveName);
    const binaryPath = path.join(storagePath, platformInfo.binaryName);

    progress.report({ message: 'Downloading...', increment: 10 });

    // Download the archive
    await downloadFile(downloadUrl, archivePath, progress);

    progress.report({ message: 'Extracting...', increment: 70 });

    // Extract the archive
    await extractArchive(archivePath, storagePath, platformInfo);

    progress.report({ message: 'Setting permissions...', increment: 90 });

    // Set executable permissions on Unix
    if (platformInfo.os !== 'win32') {
        fs.chmodSync(binaryPath, 0o755);
    }

    // Clean up archive
    if (fs.existsSync(archivePath)) {
        fs.unlinkSync(archivePath);
    }

    progress.report({ message: 'Complete!', increment: 100 });

    return binaryPath;
}

/**
 * Download a file from a URL
 */
async function downloadFile(
    url: string,
    destination: string,
    progress: vscode.Progress<{ message?: string; increment?: number }>
): Promise<void> {
    return new Promise((resolve, reject) => {
        const file = fs.createWriteStream(destination);

        const request = (url: string) => {
            https.get(url, (response) => {
                // Handle redirects
                if (response.statusCode === 301 || response.statusCode === 302) {
                    const redirectUrl = response.headers.location;
                    if (redirectUrl) {
                        request(redirectUrl);
                        return;
                    }
                }

                if (response.statusCode !== 200) {
                    reject(new Error(`Failed to download: HTTP ${response.statusCode}`));
                    return;
                }

                const totalSize = parseInt(response.headers['content-length'] || '0', 10);
                let downloadedSize = 0;

                response.on('data', (chunk: Buffer) => {
                    downloadedSize += chunk.length;
                    if (totalSize > 0) {
                        const percent = Math.round((downloadedSize / totalSize) * 50);
                        progress.report({ message: `Downloading... ${percent}%`, increment: 0 });
                    }
                });

                response.pipe(file);

                file.on('finish', () => {
                    file.close();
                    resolve();
                });
            }).on('error', (err) => {
                fs.unlink(destination, () => { }); // Delete partial file
                reject(err);
            });
        };

        request(url);
    });
}

/**
 * Extract an archive (tar.gz or zip)
 */
async function extractArchive(
    archivePath: string,
    destination: string,
    platformInfo: PlatformInfo
): Promise<void> {
    if (platformInfo.archiveExtension === 'tar.gz') {
        // Use tar package for extraction
        const tar = await import('tar');
        await tar.x({
            file: archivePath,
            cwd: destination,
        });
    } else {
        // Use built-in unzip for Windows
        await new Promise<void>((resolve, reject) => {
            exec(
                `powershell -Command "Expand-Archive -Path '${archivePath}' -DestinationPath '${destination}' -Force"`,
                (error: Error | null) => {
                    if (error) {
                        reject(error);
                    } else {
                        resolve();
                    }
                }
            );
        });
    }
}

/**
 * Get the expected version
 */
export function getExpectedVersion(): string {
    return HINDSIGHT_VERSION;
}
