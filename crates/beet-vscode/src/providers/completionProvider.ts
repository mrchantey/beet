import * as vscode from 'vscode';

export function provideCompletionItems(
    document: vscode.TextDocument,
    position: vscode.Position,
    token: vscode.CancellationToken,
    context: vscode.CompletionContext
): vscode.ProviderResult<vscode.CompletionItem[]> {
    const completionItems: vscode.CompletionItem[] = [];

    // HTML elements
    const htmlElements = [
        'div', 'span', 'p', 'h1', 'h2', 'h3', 'h4', 'h5', 'h6',
        'a', 'ul', 'ol', 'li', 'table', 'tr', 'td', 'th', 'form',
        'input', 'button', 'label', 'textarea', 'select', 'option',
        'img', 'br', 'hr', 'strong', 'em', 'blockquote', 'code'
    ];

    // CSS properties
    const cssProperties = [
        'color', 'background-color', 'font-size', 'margin', 'padding',
        'border', 'display', 'flex', 'grid', 'align-items', 'justify-content',
        'text-align', 'width', 'height', 'position', 'top', 'left', 'right', 'bottom'
    ];

    // Add HTML element completion items
    htmlElements.forEach(element => {
        completionItems.push(new vscode.CompletionItem(element, vscode.CompletionItemKind.Field));
    });

    // Add CSS property completion items
    cssProperties.forEach(property => {
        completionItems.push(new vscode.CompletionItem(property, vscode.CompletionItemKind.Property));
    });

    return completionItems;
}