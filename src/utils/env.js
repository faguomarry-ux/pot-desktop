import { type, arch as archFn, version } from '@tauri-apps/plugin-os';
import { getVersion } from '@tauri-apps/api/app';

export let osType = '';
export let arch = '';
export let osVersion = '';
export let appVersion = '';

export async function initEnv() {
    osType = { linux: 'Linux', macos: 'Darwin', windows: 'Windows_NT' }[type()] ?? type();
    arch = await archFn();
    osVersion = await version();
    appVersion = await getVersion();
}
