import * as vscode from 'vscode';
import {
    ExecuteCommandRequest,
    LanguageClient,
    LanguageClientOptions,
    ServerOptions,
    TransportKind
} from 'vscode-languageclient/node';

export interface FileOwnershipInfo {
    path: string;
    owners: Array<{ identifier: string; owner_type: string }>;
    tags: string[];
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

        try {
            const info = await this.client.sendRequest(ExecuteCommandRequest.type, {
                command: 'codeinput.getFileOwnership',
                arguments: [uri.toString()]
            }) as FileOwnershipInfo | null | undefined;

            if (info) {
                return info;
            }
        } catch (error) {
            console.error('[CodeInput Client] Error:', error);
        }

        return undefined;
    }

    public isRunning(): boolean {
        return this.client?.isRunning() === true;
    }
}