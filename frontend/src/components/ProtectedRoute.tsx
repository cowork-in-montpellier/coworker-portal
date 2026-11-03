import { useEffect, useState, type ReactNode } from 'react'
import { Navigate } from 'react-router-dom'
import { isAuthenticated, isTokenExpired, getRefreshToken, storeTokens, clearTokens } from '../auth'
import { renewToken } from '../api/auth'

type State = 'loading' | 'authenticated' | 'unauthenticated'

export function ProtectedRoute({ children }: { children: ReactNode }) {
  const [state, setState] = useState<State>(() =>
    isAuthenticated() ? 'authenticated' : 'loading'
  )

  useEffect(() => {
    if (state === 'authenticated') return

    const refreshToken = getRefreshToken()
    if (!isTokenExpired() || !refreshToken) {
      setState('unauthenticated')
      return
    }

    renewToken(refreshToken)
      .then(({ token, refresh_token }) => {
        storeTokens(token, refresh_token)
        setState('authenticated')
      })
      .catch(() => {
        clearTokens()
        setState('unauthenticated')
      })
  }, [])

  if (state === 'loading') {
    return (
      <div className="min-h-screen flex items-center justify-center">
        <span className="loading loading-spinner loading-lg" />
      </div>
    )
  }

  if (state === 'unauthenticated') {
    return <Navigate to="/login" replace />
  }

  return <>{children}</>
}
