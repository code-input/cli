import * as vscode from 'vscode';
import {
    LanguageClient,
    LanguageClientOptions,
    ServerOptions,
    TransportKind
} from 'vscode-languageclient/node';

export interface FileOwnershipInfo {
    path: string;
    owners: Array<{ identifier: string; owner_type: string }>;
    tags: Array<{ 0: string }>;
    is_unowned: boolean;
}

export class CodeInputClient {
    private client: LanguageClient | undefined;

    constructor(private binaryPath: string) {
        this.initializeClient();
    }

    private initializeClient(): void {
        const config = vscode.workspace.getConfiguration('codeinput');
        const transport = config.get<string>('lspTransport', 'stdio');
        const port = config.get<number>('lspPort', 8123);

        let serverOptions: ServerOptions;

        if (transport === 'tcp') {
            // TCP transport
            serverOptions = (() => {
                const socket = require('net').connect({ port });
                const result = {
                    reader: socket,
                    writer: socket
                };
                return Promise.resolve(result);
            }) as unknown as ServerOptions;
        } else {
            // STDIO transport (default)
            serverOptions = {
                command: this.binaryPath,
                args: ['lsp'],
                transport: TransportKind.stdio,
                options: {
                    cwd: vscode.workspace.workspaceFolders?.[0]?.uri.fsPath
                }
            };
        }

        const clientOptions: LanguageClientOptions = {
            documentSelector: [
                { scheme: 'file', pattern: '**/*' }
            ],
            synchronize: {
                fileEvents: [
                    vscode.workspace.createFileSystemWatcher('**/CODEOWNERS'),
                    vscode.workspace.createFileSystemWatcher('**/.github/CODEOWNERS'),
                    vscode.workspace.createFileSystemWatcher('**/.gitlab/CODEOWNERS')
                ]
            },
            outputChannelName: 'CodeInput LSP'
        };

        this.client = new LanguageClient(
            'codeinput',
            'CodeInput LSP',
            serverOptions,
            clientOptions
        );
    }

    public async start(): Promise<void> {
        if (this.client) {
            try {
                await this.client.start();
                console.log('CodeInput LSP client started');
            } catch (error) {
                console.error('Failed to start CodeInput LSP client:', error);
                vscode.window.showErrorMessage(
                    `Failed to start CodeInput LSP: ${error}. Make sure the 'ci-lsp' binary is installed and in your PATH.`
                );
            }
        }
    }

    public async stop(): Promise<void> {
        if (this.client) {
            await this.client.stop();
            console.log('CodeInput LSP client stopped');
        }
    }

    public async getFileOwnership(uri: vscode.Uri): Promise<FileOwnershipInfo | undefined> {
        if (!this.client) {
            return undefined;
        }

        // Use the hover request to get ownership info
        try {
            const hover = await this.client.sendRequest('textDocument/hover', {
                textDocument: { uri: uri.toString() },
                position: { line: 0, character: 0 }
            }) as any;

            if (hover && hover.contents) {
                // Parse hover contents to extract ownership info
                return this.parseHoverContents(hover.contents, uri);
            }
        } catch (error) {
            console.error('Error getting file ownership:', error);
        }

        return undefined;
    }

    private parseHoverContents(contents: any, uri: vscode.Uri): FileOwnershipInfo | undefined {
        // The LSP server returns hover contents with owners and tags
        // This is a simplified parser - in production you'd want more robust parsing
        const info: FileOwnershipInfo = {
            path: uri.fsPath,
            owners: [],
            tags: [],
            is_unowned: false
        };

        if (Array.isArray(contents)) {
            for (const item of contents) {
                if (typeof item === 'string') {
                    if (item.includes('Owners:')) {
                        const ownersMatch = item.match(/\*\*Owners:\*\* (.+)/);
                        if (ownersMatch) {
                            const ownersStr = ownersMatch[1];
                            if (ownersStr !== '(none)') {
                                info.owners = ownersStr.split(', ').map(o => ({
                                    identifier: o.replace(/`/g, ''),
                                    owner_type: 'Unknown'
                                }));
                            }
                        }
                    }
                    if (item.includes('Tags:')) {
                        const tagsMatch = item.match(/\*\*Tags:\*\* (.+)/);
                        if (tagsMatch) {
                            const tagsStr = tagsMatch[1];
                            info.tags = tagsStr.split(', ').map(t => ({
                                0: t.replace(/`#/g, '').replace(/`/g, '')
                            }));
                        }
                    }
                    if (item.includes('Warning')) {
                        info.is_unowned = true;
                    }
                }
            }
        }

        return info;
    }

    public isRunning(): boolean {
        return this.client?.isRunning() === true;
    }
}