import { fetch, Body } from './tauri-http.js';

// The legacy /translate/auth endpoint was retired. Current Edge translation
// accepts a JSON array of strings directly, without a separate token request.
export async function translateMicrosoft(text, from, to) {
    const query = { isEnterpriseClient: 'false', to };
    if (from && from !== 'auto') query.from = from;
    const response = await fetch('https://edge.microsoft.com/translate/translatetext', {
        method: 'POST',
        query,
        body: Body.json([text]),
        timeout: 20,
    });
    if (!response.ok) {
        throw new Error(`Bing Translate: HTTP ${response.status}`);
    }
    const result = response.data?.[0];
    if (typeof result?.translations?.[0]?.text !== 'string') {
        throw new Error('Bing Translate: Invalid translation response');
    }
    return result;
}
