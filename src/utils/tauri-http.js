import { fetch as nativeFetch } from '@tauri-apps/plugin-http';
import { readBinaryFile } from './tauri-fs.js';

// Keep the HTTP contract used by existing services and .potext plugins.
export const ResponseType = Object.freeze({ JSON: 1, Text: 2, Binary: 3 });
export class Body {
    constructor(type, payload) {
        this.type = type;
        this.payload = payload;
    }
    static json(value) {
        return new Body('Json', value);
    }
    static text(value) {
        return new Body('Text', value);
    }
    static bytes(value) {
        return new Body('Bytes', Array.from(value));
    }
    static form(value) {
        return new Body('Form', value);
    }
}

export async function fetch(url, options = {}) {
    const { body, query, responseType = ResponseType.JSON, timeout, ...init } = options;
    const target = new URL(url);
    for (const [key, value] of Object.entries(query ?? {})) target.searchParams.set(key, value);
    const headers = new Headers(init.headers);
    // Tauri 1 also accepted serialized Body objects; existing services and
    // external plugins use both these objects and the Body factory methods.
    if (body instanceof Body || (body && ['Json', 'Text', 'Bytes', 'Form'].includes(body.type) && 'payload' in body)) {
        switch (body.type) {
            case 'Json':
                init.body = JSON.stringify(body.payload);
                if (!headers.has('content-type')) headers.set('content-type', 'application/json');
                break;
            case 'Text':
                init.body = body.payload;
                break;
            case 'Bytes':
                init.body = new Uint8Array(body.payload);
                break;
            case 'Form': {
                if (!headers.get('content-type')?.includes('multipart/form-data')) {
                    init.body = new URLSearchParams(body.payload);
                } else {
                    const form = new FormData();
                    for (const [key, value] of Object.entries(body.payload)) {
                        if (typeof value === 'string') form.append(key, value);
                        else {
                            const file = value.file ?? value;
                            const bytes = typeof file === 'string' ? await readBinaryFile(file) : file;
                            form.append(
                                key,
                                new Blob([new Uint8Array(bytes)], { type: value.mime ?? 'application/octet-stream' }),
                                value.fileName ?? 'file'
                            );
                        }
                    }
                    headers.delete('content-type'); // fetch supplies the multipart boundary.
                    init.body = form;
                }
                break;
            }
        }
    } else if (body !== undefined) init.body = body;
    if (init.connectTimeout !== undefined) init.connectTimeout *= 1000;
    let timer;
    if (timeout !== undefined) {
        const controller = new AbortController();
        init.signal = controller.signal;
        timer = setTimeout(() => controller.abort(), timeout * 1000);
    }
    try {
        const response = await nativeFetch(target.toString(), { ...init, headers });
        let data;
        if (responseType === ResponseType.Binary) {
            data = Array.from(new Uint8Array(await response.arrayBuffer()));
        } else {
            const text = await response.text();
            if (responseType === ResponseType.Text) data = text;
            else {
                try {
                    data = JSON.parse(text);
                } catch {
                    // Providers often send HTML for 429/403/502 responses. Preserve
                    // status and body so callers can report the actual HTTP failure.
                    if (!response.ok) data = text;
                    else throw new Error(`Invalid JSON response (HTTP ${response.status}; ${response.headers.get('content-type') ?? 'unknown content type'})`);
                }
            }
        }
        const rawHeaders = Object.fromEntries([...response.headers].map(([key, value]) => [key, [value]]));
        if (response.headers.getSetCookie?.().length) rawHeaders['set-cookie'] = response.headers.getSetCookie();
        return {
            url: response.url,
            status: response.status,
            ok: response.ok,
            headers: Object.fromEntries(response.headers),
            rawHeaders,
            data,
        };
    } finally {
        clearTimeout(timer);
    }
}

export async function getClient(options = {}) {
    return {
        drop: async () => {},
        request: ({ url, ...init }) => fetch(url, { ...options, ...init }),
        get: (url, init) => fetch(url, { ...options, ...init, method: 'GET' }),
        post: (url, body, init) => fetch(url, { ...options, ...init, method: 'POST', body }),
        put: (url, body, init) => fetch(url, { ...options, ...init, method: 'PUT', body }),
        patch: (url, body, init) => fetch(url, { ...options, ...init, method: 'PATCH', body }),
        delete: (url, init) => fetch(url, { ...options, ...init, method: 'DELETE' }),
    };
}
