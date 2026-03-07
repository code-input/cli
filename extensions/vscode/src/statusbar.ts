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

        if (fileUri.scheme !== 'file') {
            this.statusBarItem.hide();
            return;
        }

        try {
            const info = await this.client.getFileOwnership(fileUri);

            if (info && !info.is_unowned && info.owners.length > 0) {
                this.statusBarItem.text = '$(lock)';
                this.statusBarItem.tooltip = 'Has CODEOWNERS';
                this.statusBarItem.backgroundColor = undefined;
            } else {
                this.statusBarItem.text = '$(alert)';
                this.statusBarItem.tooltip = 'No CODEOWNERS';
                this.statusBarItem.backgroundColor = new vscode.ThemeColor('statusBarItem.warningBackground');
            }
            this.statusBarItem.show();
        } catch (error) {
            console.error('Error updating status bar:', error);
            this.statusBarItem.text = '$(alert)';
            this.statusBarItem.tooltip = 'No CODEOWNERS';
            this.statusBarItem.backgroundColor = new vscode.ThemeColor('statusBarItem.warningBackground');
            this.statusBarItem.show();
        }
    }

    public dispose(): void {
        this.statusBarItem.dispose();
        this.disposables.forEach(d => d.dispose());
    }
}