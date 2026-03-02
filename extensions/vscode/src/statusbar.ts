import * as vscode from 'vscode';
import { CodeInputClient } from './client';

export class StatusBarManager implements vscode.Disposable {
    private statusBarItem: vscode.StatusBarItem;
    private client: CodeInputClient;
    private disposables: vscode.Disposable[] = [];

    constructor(client: CodeInputClient) {
        this.client = client;
        this.statusBarItem = vscode.window.createStatusBarItem(
            vscode.StatusBarAlignment.Right,
            100
        );
        this.statusBarItem.command = 'codeinput.showInfo';

        // Update when active editor changes
        this.disposables.push(
            vscode.window.onDidChangeActiveTextEditor(() => {
                this.update();
            })
        );

        // Update when document is saved (cache might have changed)
        this.disposables.push(
            vscode.workspace.onDidSaveTextDocument(() => {
                this.update();
            })
        );

        // Initial update
        this.update();
    }

    public show(): void {
        this.statusBarItem.show();
    }

    public hide(): void {
        this.statusBarItem.hide();
    }

    private async update(): Promise<void> {
        const editor = vscode.window.activeTextEditor;
        if (!editor) {
            this.statusBarItem.hide();
            return;
        }

        const config = vscode.workspace.getConfiguration('codeinput');
        if (!config.get<boolean>('showInStatusBar', true)) {
            this.statusBarItem.hide();
            return;
        }

        const fileUri = editor.document.uri;

        // Only show for file:// URIs
        if (fileUri.scheme !== 'file') {
            this.statusBarItem.hide();
            return;
        }

        try {
            const info = await this.client.getFileOwnership(fileUri);

            if (info) {
                if (info.is_unowned || info.owners.length === 0) {
                    this.statusBarItem.text = '$(warning)  Unowned';
                    this.statusBarItem.tooltip = 'This file has no CODEOWNERS assignment\nClick to see details';
                    this.statusBarItem.backgroundColor = new vscode.ThemeColor('statusBarItem.warningBackground');
                } else {
                    const ownerNames = info.owners.map(o => o.identifier).join(', ');
                    const ownerInitials = info.owners
                        .map(o => this.getInitials(o.identifier))
                        .join('');

                    this.statusBarItem.text = `$(lock)  ${ownerInitials}`;
                    this.statusBarItem.tooltip = `Owners: ${ownerNames}\nClick to see details`;
                    this.statusBarItem.backgroundColor = undefined;
                }

                this.statusBarItem.show();
            } else {
                // No CODEOWNERS info available (file not in cache)
                this.statusBarItem.hide();
            }
        } catch (error) {
            console.error('Error updating status bar:', error);
            this.statusBarItem.hide();
        }
    }

    private getInitials(identifier: string): string {
        // Extract initials from owner identifier
        // @org/team -> T, @username -> U, email@domain.com -> E

        if (identifier.startsWith('@')) {
            // Handle @org/team format
            const parts = identifier.split('/');
            if (parts.length > 1) {
                return parts[parts.length - 1].charAt(0).toUpperCase();
            }
            // Handle @username format
            return identifier.charAt(1).toUpperCase();
        }

        if (identifier.includes('@')) {
            // Email format
            return identifier.charAt(0).toUpperCase();
        }

        // Fallback
        return identifier.charAt(0).toUpperCase();
    }

    public dispose(): void {
        this.statusBarItem.dispose();
        this.disposables.forEach(d => d.dispose());
    }
}