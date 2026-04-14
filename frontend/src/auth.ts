interface TokenPayload {
  sub: number
  username: string
  first_name: string
  django_session: string | null
  exp: number
  iat: number
}

function decodeJwtPayload(part: string): string {
  // base64url → base64
  const base64 = part.replace(/-/g, '+').replace(/_/g, '/')
  const binary = atob(base64)
  const bytes = Uint8Array.from(binary, c => c.charCodeAt(0))
  return new TextDecoder().decode(bytes)
}

export function getTokenPayload(): TokenPayload | null {
  const token = localStorage.getItem('token')
  if (!token) return null
  try {
    return JSON.parse(decodeJwtPayload(token.split('.')[1])) as TokenPayload
  } catch {
    return null
  }
}

export function isAuthenticated(): boolean {
  const payload = getTokenPayload()
  if (!payload) return false
  return payload.exp * 1000 > Date.now()
}

export function hasDjangoSession(): boolean {
  return !!getTokenPayload()?.django_session
}

export function logout() {
  localStorage.removeItem('token')
}
