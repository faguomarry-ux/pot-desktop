import { Store } from '@tauri-apps/plugin-store';
import { appConfigDir, join } from '@tauri-apps/api/path';
import { watch } from './tauri-fs.js';
import { invoke } from '@tauri-apps/api/core';

export let store;

export async function initStore() {
    const appConfigDirPath = await appConfigDir();
    const appConfigPath = await join(appConfigDirPath, 'config.json');
    store = await Store.load(appConfigPath, { autoSave: false });
    const _ = await watch(appConfigPath, async () => {
        await store.reload();
        await invoke('reload_store');
    });
}
