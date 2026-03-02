import * as vscode from 'vscode';
import * as https from 'https';
import * as fs from 'fs';
import * as path from 'path';
import * as os from 'os';
import { spawn } from 'child_process';

export class BinaryDownloader {
    private readonly baseUrl = 'https://github.com/code-input/cli/releases/latest/download';

    constructor(private context: vscode.ExtensionContext) {}

    async ensureBinary(): Promise<string | undefined> {
        const binaryName = this.getBinaryName();
        const binaryPath = this.getBinaryPath(binaryName);

        // Check if binary already exists and is executable
        if (fs.existsSync(binaryPath)) {
            if (await this.isBinaryWorking(binaryPath)) {
                console.log(`[CodeInput] Found working binary at ${binaryPath}`);
                return binaryPath;
            } else {
                console.log(`[CodeInput] Binary exists at ${binaryPath} but not working, removing...`);
                fs.unlinkSync(binaryPath);
            }
        }

        // Download the binary
        return await this.downloadBinary(binaryName, binaryPath);
    }

    private async isBinaryWorking(binaryPath: string): Promise<boolean> {
        return new Promise((resolve) => {
            const proc = spawn(binaryPath, ['lsp', '--help']);
            proc.on('error', () => resolve(false));
            proc.on('exit', (code) => resolve(code === 0));
            // Timeout after 2 seconds
            setTimeout(() => {
                proc.kill();
                resolve(false);
            }, 2000);
        });
    }

    private getBinaryName(): string {
        const platform = os.platform();
        const arch = os.arch();

        let platformSuffix: string;
        switch (platform) {
            case 'linux': platformSuffix = 'linux'; break;
            case 'darwin': platformSuffix = 'darwin'; break;
            case 'win32': platformSuffix = 'windows'; break;
            default: platformSuffix = 'linux'; // fallback
        }

        let archSuffix: string;
        switch (arch) {
            case 'x64': archSuffix = 'x64'; break;
            case 'arm64': archSuffix = 'arm64'; break;
            default: archSuffix = 'x64'; // fallback
        }

        const ext = platform === 'win32' ? '.exe' : '';
        return `ci-lsp-${platformSuffix}-${archSuffix}${ext}`;
    }

    private getBinaryPath(binaryName: string): string {
        const extensionDir = this.context.globalStorageUri.fsPath;
        // Ensure directory exists
        if (!fs.existsSync(extensionDir)) {
            fs.mkdirSync(extensionDir, { recursive: true });
        }
        return path.join(extensionDir, binaryName);
    }

    private async downloadBinary(binaryName: string, destPath: string): Promise<string | undefined> {
        const url = `${this.baseUrl}/${binaryName}`;

        return await vscode.window.withProgress({
            location: vscode.ProgressLocation.Notification,
            title: `Downloading codeinput-lsp (${binaryName})...`,
            cancellable: false
        }, async (progress) => {
            try {
                await this.downloadFile(url, destPath, progress);

                // Make executable on Unix
                if (os.platform() !== 'win32') {
                    fs.chmodSync(destPath, 0o755);
                }

                vscode.window.showInformationMessage('codeinput-lsp downloaded successfully!');
                return destPath;
            } catch (error) {
                vscode.window.showErrorMessage(`Failed to download codeinput-lsp: ${error}`);
                return undefined;
            }
        });
    }

    private downloadFile(url: string, destPath: string, progress: vscode.Progress<{ message?: string; increment?: number }>): Promise<void> {
        return new Promise((resolve, reject) => {
            const file = fs.createWriteStream(destPath);

            https.get(url, { headers: { 'User-Agent': 'codeinput-vscode' } }, (response) => {
                // Handle redirects
                if (response.statusCode === 302 || response.statusCode === 301) {
                    const redirectUrl = response.headers.location;
                    if (redirectUrl) {
                        file.close();
                        fs.unlinkSync(destPath);
                        this.downloadFile(redirectUrl, destPath, progress).then(resolve).catch(reject);
                        return;
                    }
                }

                if (response.statusCode !== 200) {
                    reject(new Error(`Download failed with status ${response.statusCode}`));
                    return;
                }

                const totalBytes = parseInt(response.headers['content-length'] || '0', 10);
                let downloadedBytes = 0;

                response.on('data', (chunk: Buffer) => {
                    downloadedBytes += chunk.length;
                    if (totalBytes > 0) {
                        const percent = Math.round((downloadedBytes / totalBytes) * 100);
                        progress.report({ message: `${percent}%` });
                    }
                });

                response.pipe(file);
                file.on('finish', () => {
                    file.close();
                    resolve();
                });
            }).on('error', (err) => {
                fs.unlinkSync(destPath);
                reject(err);
            });
        });
    }
}
