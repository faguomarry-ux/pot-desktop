import * as fs from '@tauri-apps/plugin-fs';
import * as pathApi from '@tauri-apps/api/path';
const { join } = pathApi;
const resolveDirectory = (dir) => pathApi[pathApi.BaseDirectory[dir].replace(/^./, (c) => c.toLowerCase()) + 'Dir']();

export { BaseDirectory } from '@tauri-apps/plugin-fs';
const optionsV2 = ({ dir, ...options } = {}) => ({ ...options, ...(dir === undefined ? {} : { baseDir: dir }) });
export const readBinaryFile = (path, options) => fs.readFile(path, optionsV2(options));
export const readTextFile = (path, options) => fs.readTextFile(path, optionsV2(options));
export const exists = (path, options) => fs.exists(path, optionsV2(options));
export const removeDir = (path, options) => fs.remove(path, optionsV2(options));
// Reading the watched config during reload must not trigger another reload.
export const watch = (path, callback, options) =>
    fs.watch(
        path,
        (event) => {
            if (typeof event.type === 'object' && ('access' in event.type || event.type.modify?.kind === 'metadata'))
                return;
            callback(event);
        },
        options
    );
export async function readDir(path, options = {}) {
    const base = options.dir === undefined ? path : await join(await resolveDirectory(options.dir), path);
    const entries = await fs.readDir(path, optionsV2(options));
    return Promise.all(
        entries.map(async (entry) => {
            const entryPath = await join(base, entry.name);
            return {
                ...entry,
                path: entryPath,
                ...(entry.isDirectory && options.recursive
                    ? { children: await readDir(entryPath, { recursive: true }) }
                    : {}),
            };
        })
    );
}
