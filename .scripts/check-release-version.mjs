import { readFileSync } from 'node:fs';
const pkg = JSON.parse(readFileSync('package.json', 'utf8'));
const config = JSON.parse(readFileSync('src-tauri/tauri.conf.json', 'utf8'));
if (pkg.version !== config.version) throw new Error('package.json and tauri.conf.json versions must match');
const ref = process.env.GITHUB_REF ?? '';
if (ref.startsWith('refs/tags/v') && ref.slice('refs/tags/v'.length) !== pkg.version) {
    throw new Error(`Release tag must match v${pkg.version}; update both version files before tagging`);
}
console.log(`Package version: ${pkg.version}`);
