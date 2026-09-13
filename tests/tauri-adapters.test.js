import assert from 'node:assert/strict';
import { mock, test } from 'node:test';

let request;
let response;
let fileCall;
let watchCallback;
mock.module('@tauri-apps/plugin-http', {
    namedExports: {
        fetch: async (url, init) => {
            request = { url, init };
            return response;
        },
    },
});
mock.module('@tauri-apps/plugin-fs', {
    namedExports: {
        BaseDirectory: { AppConfig: 13, AppCache: 16 },
        readFile: async (path, options) => {
            fileCall = { path, options };
            return new Uint8Array([1, 2, 3]);
        },
        readTextFile: async () => 'text',
        exists: async () => true,
        remove: async (path, options) => {
            fileCall = { path, options };
        },
        watch: async (_path, callback) => {
            watchCallback = callback;
            return () => {};
        },
        readDir: async () => [{ name: 'plugin-demo', isDirectory: true, isFile: false, isSymlink: false }],
    },
});
mock.module('@tauri-apps/api/path', {
    namedExports: {
        BaseDirectory: { 13: 'AppConfig', 16: 'AppCache' },
        appConfigDir: async () => '/config/pot',
        appCacheDir: async () => '/cache/pot',
        join: async (...parts) => parts.join('/'),
    },
});
const { fetch, Body, ResponseType } = await import('../src/utils/tauri-http.js');
const fs = await import('../src/utils/tauri-fs.js');

test('JSON requests preserve service response.data, headers and query parameters', async () => {
    response = new Response('{"translation":"你好"}', { headers: { 'x-test': 'yes' } });
    const result = await fetch('https://example.com/translate?existing=1', {
        method: 'POST',
        query: { q: 'hello world' },
        body: Body.json({ text: 'hello' }),
    });
    assert.equal(new URL(request.url).searchParams.get('existing'), '1');
    assert.equal(new URL(request.url).searchParams.get('q'), 'hello world');
    assert.equal(request.init.headers.get('content-type'), 'application/json');
    assert.equal(request.init.body, '{"text":"hello"}');
    assert.deepEqual(result.data, { translation: '你好' });
    assert.equal(result.headers['x-test'], 'yes');
});

test('form bodies default to URL encoding as in Tauri 1', async () => {
    response = new Response('{}');
    await fetch('https://example.com', { method: 'POST', body: Body.form({ q: 'a & b' }) });
    assert.equal(request.init.body.toString(), 'q=a+%26+b');
});

test('OCR multipart uploads preserve file bytes and let fetch generate the boundary', async () => {
    response = new Response('{}');
    await fetch('https://example.com', {
        method: 'POST',
        headers: { 'Content-Type': 'multipart/form-data' },
        body: Body.form({ file: { file: [137, 80, 78, 71], fileName: 'image.png', mime: 'image/png' } }),
    });
    assert.equal(request.init.headers.has('content-type'), false);
    const file = request.init.body.get('file');
    assert.equal(file.name, 'image.png');
    assert.deepEqual([...new Uint8Array(await file.arrayBuffer())], [137, 80, 78, 71]);
});

test('dictionary HTML, audio bytes and HTTP errors retain v1 response types', async () => {
    response = new Response('<html>word</html>', { status: 404 });
    const html = await fetch('https://example.com', { responseType: ResponseType.Text });
    assert.equal(html.ok, false);
    assert.equal(html.status, 404);
    assert.equal(html.data, '<html>word</html>');
    response = new Response(new Uint8Array([0, 128, 255]));
    assert.deepEqual((await fetch('https://example.com', { responseType: ResponseType.Binary })).data, [0, 128, 255]);
});

test('file adapters translate base directory options and restore directory entry paths', async () => {
    await fs.readBinaryFile('shot.png', { dir: fs.BaseDirectory.AppCache });
    assert.deepEqual(fileCall, { path: 'shot.png', options: { baseDir: 16 } });
    await fs.removeDir('plugins/demo', { dir: fs.BaseDirectory.AppConfig, recursive: true });
    assert.deepEqual(fileCall.options, { baseDir: 13, recursive: true });
    const entries = await fs.readDir('plugins', { dir: fs.BaseDirectory.AppConfig });
    assert.equal(entries[0].path, '/config/pot/plugins/plugin-demo');
});

test('config file watching ignores reads and forwards content changes', async () => {
    const changes = [];
    await fs.watch('/config/pot/config.json', (event) => changes.push(event));
    watchCallback({ type: { access: { kind: 'open', mode: 'read' } } });
    watchCallback({ type: { modify: { kind: 'metadata', mode: 'access-time' } } });
    assert.equal(changes.length, 0);
    watchCallback({ type: { modify: { kind: 'data', mode: 'content' } } });
    assert.equal(changes.length, 1);
});


test('HTML rate-limit responses preserve HTTP status instead of raising a JSON SyntaxError', async () => {
    response = new Response('<html>Too many requests</html>', { status: 429, headers: { 'content-type': 'text/html' } });
    const result = await fetch('https://example.com/translate');
    assert.equal(result.status, 429);
    assert.equal(result.ok, false);
    assert.equal(result.data, '<html>Too many requests</html>');
});

test('successful non-JSON responses report status and content type', async () => {
    response = new Response('<html>Unexpected login page</html>', { headers: { 'content-type': 'text/html' } });
    await assert.rejects(fetch('https://example.com/translate'), /Invalid JSON response \(HTTP 200; text\/html\)/);
});

const { detectBeforeTranslate } = await import('../src/utils/detect-before-translate.js');
test('language detection preserves a successful result', async () => {
    assert.equal(await detectBeforeTranslate(async () => 'zh_cn', '你好'), 'zh_cn');
});
test('failed language detection does not prevent submitting translation', async () => {
    assert.equal(await detectBeforeTranslate(async () => { throw new Error('HTTP 429'); }, 'Hello'), '');
});
test('unresponsive language detection does not prevent submitting translation', async () => {
    assert.equal(await detectBeforeTranslate(() => new Promise(() => {}), 'Hello', 10), '');
});

test('serialized Tauri 1 JSON bodies used by Bing are sent as JSON', async () => {
    response = new Response('[]', { status: 200 });
    await fetch('https://example.com/translate', {
        method: 'POST', body: { type: 'Json', payload: [{ Text: 'Hello' }] },
    });
    assert.equal(request.init.body, '[{"Text":"Hello"}]');
    assert.equal(request.init.headers.get('content-type'), 'application/json');
});
test('serialized Tauri 1 text bodies preserve signed request bytes', async () => {
    response = new Response('{}', { status: 200 });
    await fetch('https://example.com/translate', {
        method: 'POST', body: { type: 'Text', payload: '{"text":"你好"}' },
    });
    assert.equal(request.init.body, '{"text":"你好"}');
});

const { translateMicrosoft } = await import('../src/utils/microsoft-translate.js');
test('Bing uses the current endpoint with automatic detection and a string array', async () => {
    response = new Response(JSON.stringify([{ detectedLanguage: { language: 'en' }, translations: [{ text: '你好，世界' }] }]));
    const result = await translateMicrosoft('Hello world', 'auto', 'zh-Hans');
    assert.equal(result.translations[0].text, '你好，世界');
    assert.equal(result.detectedLanguage.language, 'en');
    assert.equal(new URL(request.url).pathname, '/translate/translatetext');
    assert.equal(new URL(request.url).searchParams.has('from'), false);
    assert.equal(request.init.body, '["Hello world"]');
    response = new Response(JSON.stringify([{ translations: [{ text: 'Hello' }] }]));
    await translateMicrosoft('你好', 'zh-Hans', 'en');
    assert.equal(new URL(request.url).searchParams.get('from'), 'zh-Hans');
});
test('Bing reports HTTP failures and malformed successful responses', async () => {
    response = new Response('<html>Unavailable</html>', { status: 503 });
    await assert.rejects(translateMicrosoft('Hello', 'auto', 'zh-Hans'), /HTTP 503/);
    response = new Response('{}');
    await assert.rejects(translateMicrosoft('Hello', 'auto', 'zh-Hans'), /Invalid translation response/);
});
