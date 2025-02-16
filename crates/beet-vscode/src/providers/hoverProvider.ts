import * as vscode from 'vscode';

export class HoverProvider implements vscode.HoverProvider {
    provideHover(
        document: vscode.TextDocument,
        position: vscode.Position,
        token: vscode.CancellationToken
    ): vscode.ProviderResult<vscode.Hover> {
        const wordRange = document.getWordRangeAtPosition(position);
        const word = document.getText(wordRange);

        // Example hover information for HTML and CSS elements
        const hoverInfo = this.getHoverInfo(word);
        if (hoverInfo) {
            return new vscode.Hover(hoverInfo);
        }
        return null;
    }

    private getHoverInfo(word: string): string | null {
        const htmlElements: { [key: string]: string } = {
            'div': 'The <div> element is a block container.',
            'text': 'The <text> element is used for text content.',
            'style': 'The <style> element is used to define CSS styles.',
            // Add more elements as needed
        };

        return htmlElements[word] || null;
    }
}