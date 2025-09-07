export async function hashString(str: string): Promise<string> {
    const enc = new TextEncoder().encode(str)
    const buf = await crypto.subtle.digest('SHA-256', enc)
    return [...new Uint8Array(buf)].map(b => b.toString(16).padStart(2, '0')).join('')
}