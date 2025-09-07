// Minimal stub — replace with a proper CDR decoder or switch server to JSON encoding during early dev
export function decodeRos2Cdr(typeName: string, schemaText: string, buf: ArrayBuffer): any {
    // For MVP, we won’t truly decode CDR here. Two options:
    // 1) Run Foxglove Bridge in JSON mode (encoding json) -> then this path won’t be used.
    // 2) Later: implement CDR parsing or integrate an existing decoder.
    // Return a placeholder to unblock UI wiring
    return { _type: typeName, _note: 'CDR decode not implemented in MVP', _bytes: buf.byteLength }
}