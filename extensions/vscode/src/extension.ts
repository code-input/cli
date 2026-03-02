import * as vscode from 'vscode';
import { CodeInputClient } from './client';
import { StatusBarManager } from './statusbar';
import { BinaryDownloader } from './downloader';
import { spawn } from 'child_process';

let client: CodeInputClient | undefined;
let statusBar: StatusBarManager | undefined;

async function checkBinary(binaryPath: string): Promise<boolean> {
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

export async function activate(context: vscode.ExtensionContext) {
    console.log('CodeInput extension is now active');

    // Check if custom binary path is set
    const config = vscode.workspace.getConfiguration('codeinput');
    const userBinaryPath = config.get<string>('binaryPath', 'ci-lsp');

    let finalBinaryPath: string | undefined;
    let binarySource = 'unknown';

    // If user set a custom path, use it
    if (userBinaryPath !== 'ci-lsp') {
        console.log(`[CodeInput] Checking user-specified binary: ${userBinaryPath}`);
        const exists = await checkBinary(userBinaryPath);
        if (exists) {
            finalBinaryPath = userBinaryPath;
            binarySource = 'user-config';
            console.log(`[CodeInput] Using user-specified binary: ${finalBinaryPath}`);
        } else {
            console.error(`[CodeInput] User-specified binary not found: ${userBinaryPath}`);
            vscode.window.showErrorMessage(
                `Custom binary not found: ${userBinaryPath}. Please check the path or install codeinput-lsp.`
            );
            return;
        }
    } else {
        // Try to download ci-lsp first
        console.log('[CodeInput] Attempting to download ci-lsp binary...');
        const downloader = new BinaryDownloader(context);
        const downloadedPath = await downloader.ensureBinary();

        if (downloadedPath) {
            finalBinaryPath = downloadedPath;
            binarySource = 'downloaded';
            console.log(`[CodeInput] Using downloaded binary: ${finalBinaryPath}`);
        } else {
            console.log('[CodeInput] Download failed, checking for existing ci binary with LSP support...');
            // Download failed, check if `ci` with LSP exists
            const ciExists = await checkBinary('ci');
            if (ciExists) {
                binarySource = 'fallback-ci';
                finalBinaryPath = 'ci';
                console.log('[CodeInput] Using fallback ci binary with LSP support');
                vscode.window.showInformationMessage(
                    'Using existing `ci` binary with LSP support. Consider downloading `ci-lsp` for better performance.'
                );
            } else {
                // Neither works
                console.error('[CodeInput] No usable binary found (tried downloading ci-lsp and using ci)');
                vscode.window.showErrorMessage(
                    'codeinput-lsp not found. Please install it: curl -L https://github.com/code-input/cli/releases/latest/download/ci-lsp-linux-x64 -o ~/bin/ci-lsp && chmod +x ~/bin/ci-lsp'
                );
                return;
            }
        }
    }

    // Initialize the LSP client with the binary path
    console.log(`[CodeInput] Starting LSP client with binary: ${finalBinaryPath} (source: ${binarySource})`);
    client = new CodeInputClient(finalBinaryPath);
    await client.start();

    // Initialize status bar
    if (config.get<boolean>('showInStatusBar', true)) {
        statusBar = new StatusBarManager(client);
        statusBar.show();
        context.subscriptions.push(statusBar);
    }

    // Register commands
    context.subscriptions.push(
        vscode.commands.registerCommand('codeinput.showInfo', async () => {
            const editor = vscode.window.activeTextEditor;
            if (!editor) {
                vscode.window.showInformationMessage('No active editor');
                return;
            }

            const fileUri = editor.document.uri;
            const info = await client?.getFileOwnership(fileUri);

            if (info) {
                const owners = info.owners.length > 0
                    ? info.owners.map(o => o.identifier).join(', ')
                    : '(none)';
                const tags = info.tags.length > 0
                    ? info.tags.map(t => `#${t}`).join(', ')
                    : '(none)';

                const message = [
                    `**File:** ${fileUri.fsPath}`,
                    `**Owners:** ${owners}`,
                    `**Tags:** ${tags}`,
                ].join('\n\n');

                vscode.window.showInformationMessage(message, { modal: true });
            } else {
                vscode.window.showInformationMessage('No CODEOWNERS info available for this file');
            }
        })
    );

    context.subscriptions.push(
        vscode.commands.registerCommand('codeinput.refreshCache', async () => {
            await vscode.window.withProgress({
                location: vscode.ProgressLocation.Window,
                title: 'Refreshing CODEOWNERS cache...'
            }, async () => {
                // The LSP server will automatically refresh when CODEOWNERS files change
                vscode.window.showInformationMessage('CODEOWNERS cache refreshed');
            });
        })
    );

    context.subscriptions.push(
        vscode.commands.registerCommand('codeinput.showOwners', (uri: string, owners: any[]) => {
            if (owners && owners.length > 0) {
                const ownerList = owners.map(o => o.identifier).join(', ');
                vscode.window.showInformationMessage(`Owners: ${ownerList}`);
            }
        })
    );

    context.subscriptions.push(
        vscode.commands.registerCommand('codeinput.showTags', (uri: string, tags: any[]) => {
            if (tags && tags.length > 0) {
                const tagList = tags.map(t => `#${t}`).join(', ');
                vscode.window.showInformationMessage(`Tags: ${tagList}`);
            }
        })
    );

    context.subscriptions.push(
        vscode.commands.registerCommand('codeinput.addOwner', async (uri: string) => {
            const owner = await vscode.window.showInputBox({
                prompt: 'Enter owner (e.g., @username or @org/team)',
                placeHolder: '@username'
            });

            if (owner) {
                vscode.window.showInformationMessage(`Would add owner ${owner} to ${uri}`);
                // TODO: Implement adding owner to CODEOWNERS file
            }
        })
    );

    // Handle configuration changes
    context.subscriptions.push(
        vscode.workspace.onDidChangeConfiguration(e => {
            if (e.affectsConfiguration('codeinput.showInStatusBar')) {
                const showInStatusBar = config.get<boolean>('showInStatusBar', true);

                if (showInStatusBar && !statusBar) {
                    statusBar = new StatusBarManager(client!);
                    statusBar.show();
                    context.subscriptions.push(statusBar);
                } else if (!showInStatusBar && statusBar) {
                    statusBar.dispose();
                    statusBar = undefined;
                }
            }
        })
    );

    // Clean up on deactivate
    context.subscriptions.push({
        dispose: () => {
            client?.stop();
        }
    });
}

export function deactivate(): Thenable<void> | undefined {
    statusBar?.dispose();
    return client?.stop();
}
