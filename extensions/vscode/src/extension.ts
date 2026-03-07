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
        setTimeout(() => {
            proc.kill();
            resolve(false);
        }, 2000);
    });
}

export async function activate(context: vscode.ExtensionContext) {
    console.log('CodeInput extension is now active');

    const config = vscode.workspace.getConfiguration('codeinput');
    const binaryPath = config.get<string>('binaryPath', 'ci-lsp');

    let finalBinaryPath: string | undefined;
    let binarySource = 'unknown';

    const exists = await checkBinary(binaryPath);
    if (exists) {
        finalBinaryPath = binaryPath;
        binarySource = 'config';
        console.log(`[CodeInput] Using binary from config: ${finalBinaryPath}`);
    } else {
        console.log(`[CodeInput] Binary not found at ${binaryPath}, attempting to download...`);
        const downloader = new BinaryDownloader(context);
        const downloadedPath = await downloader.ensureBinary();

        if (downloadedPath) {
            finalBinaryPath = downloadedPath;
            binarySource = 'downloaded';
            console.log(`[CodeInput] Using downloaded binary: ${finalBinaryPath}`);
        } else {
            console.error('[CodeInput] No usable binary found');
            vscode.window.showErrorMessage(
                'codeinput binary not found. Install it or set codeinput.binaryPath in settings.'
            );
            return;
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
