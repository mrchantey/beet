// The main entry point for the VS Code extension
import * as vscode from 'vscode';
import { registerCompletionProvider } from './providers/completionProvider';
import { registerHoverProvider } from './providers/hoverProvider';

export function activate(context: vscode.ExtensionContext) {
    // Register the completion provider for rsx! macros
    const completionProvider = registerCompletionProvider();
    context.subscriptions.push(completionProvider);

    // Register the hover provider for rsx! macros
    const hoverProvider = registerHoverProvider();
    context.subscriptions.push(hoverProvider);
}

export function deactivate() {}