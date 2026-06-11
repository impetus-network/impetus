/** Encode a cursor payload as url-safe base64. */
export function encodeCursor(payload: object): string {
  return Buffer.from(JSON.stringify(payload)).toString("base64url");
}

/** Decode a base64url cursor back to its typed shape. */
export function decodeCursor<T>(s: string): T {
  return JSON.parse(Buffer.from(s, "base64url").toString("utf8")) as T;
}
