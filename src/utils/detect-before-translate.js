// Language detection improves the target-language choice, but must not prevent
// translation services from using their own automatic language detection.
export async function detectBeforeTranslate(detect, text, timeoutMs = 5000) {
    let timer;
    try {
        return await Promise.race([
            Promise.resolve().then(() => detect(text)),
            new Promise((_, reject) => {
                timer = setTimeout(() => reject(new Error('Language detection timed out')), timeoutMs);
            }),
        ]);
    } catch (error) {
        console.warn('Language detection unavailable; continuing with automatic detection:', error);
        return '';
    } finally {
        clearTimeout(timer);
    }
}
