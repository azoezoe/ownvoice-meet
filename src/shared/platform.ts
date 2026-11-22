// WKWebView reports Macintosh on macOS; keep Node-based tests safe to import.
export const IS_MAC = typeof navigator !== 'undefined' && /Macintosh|Mac OS X/.test(navigator.userAgent);

export function shortcutLabel(text: string, mac: boolean): string {
  return mac ? text.replaceAll('Ctrl+Shift+', '⌘+Shift+').replaceAll('系統匣圖示', '選單列圖示') : text;
}

/** The HTML uses Windows labels; the actual bindings use CommandOrControl on both platforms. */
export function adaptPlatformText(): void {
  const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT);
  while (walker.nextNode()) {
    const node = walker.currentNode;
    if (node.parentElement?.closest('script, style')) continue;
    node.textContent = shortcutLabel(node.textContent ?? '', IS_MAC);
  }
  for (const el of document.querySelectorAll<HTMLElement>('[title]')) el.title = shortcutLabel(el.title, IS_MAC);
}
