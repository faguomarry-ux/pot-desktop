import { translateMicrosoft } from '../../../utils/microsoft-translate.js';

export async function translate(text, from, to) {
    const result = await translateMicrosoft(text, from, to);
    return result.translations[0].text.trim();
}

export * from './Config';
export * from './info';
