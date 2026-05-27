import { useState } from 'react'
import { Link, useLocation } from 'react-router-dom'
import { getTokenPayload, isAuthenticated, logout } from '../auth'
import { type ThemeId, getTheme, setTheme } from '../lib/theme'

export function Navbar() {
  const user = getTokenPayload()
  const authenticated = isAuthenticated()
  const [theme, setThemeState] = useState<ThemeId>(getTheme)
  const location = useLocation()

  const handleThemeChange = (t: ThemeId) => {
    setTheme(t)
    setThemeState(t)
  }

  const handleLogout = () => {
    logout()
    window.location.href = '/login'
  }

  const navLink = (to: string, label: string) => {
    const active = location.pathname === to || (to === '/dashboard' && location.pathname.startsWith('/bills'))
    return (
      <Link
        to={to}
        className={`btn btn-ghost btn-sm ${active ? 'btn-active' : ''}`}
      >
        {label}
      </Link>
    )
  }

  return (
    <div className="navbar bg-base-100 border-b border-base-200 px-4">
      <div className="navbar-start gap-4">
        <Link to={authenticated ? '/dashboard' : '/login'}>
          <img src="/logo-default.png" alt="Coworking" className="h-8" />
        </Link>
        {authenticated && (
          <nav className="flex items-center gap-1">
            {navLink('/dashboard', 'Factures')}
            {navLink('/calendar', 'Calendrier')}
          </nav>
        )}
      </div>
      <div className="navbar-end gap-3">
        <label className="flex items-center gap-1 cursor-pointer" title={theme === 'corporate' ? 'Passer en mode sombre' : 'Passer en mode clair'}>
          <span className="text-sm">☀️</span>
          <input
            type="checkbox"
            className="toggle toggle-sm"
            checked={theme === 'dim'}
            onChange={e => handleThemeChange(e.target.checked ? 'dim' : 'corporate')}
            aria-label="Changer de thème"
          />
          <span className="text-sm">🌙</span>
        </label>
        {authenticated && user ? (
          <div className="dropdown dropdown-end">
            <div tabIndex={0} role="button" className="btn btn-ghost btn-sm gap-1">
              <span className="text-sm">{user.first_name}</span>
              <svg xmlns="http://www.w3.org/2000/svg" className="h-3 w-3 opacity-60" viewBox="0 0 20 20" fill="currentColor">
                <path fillRule="evenodd" d="M5.293 7.293a1 1 0 011.414 0L10 10.586l3.293-3.293a1 1 0 111.414 1.414l-4 4a1 1 0 01-1.414 0l-4-4a1 1 0 010-1.414z" clipRule="evenodd" />
              </svg>
            </div>
            <ul tabIndex={0} className="dropdown-content menu bg-base-100 rounded-box shadow-lg border border-base-200 z-50 w-44 p-1 mt-1">
              <li><Link to="/profile">Mon profil</Link></li>
              <li><button onClick={handleLogout}>Se déconnecter</button></li>
            </ul>
          </div>
        ) : (
          <Link to="/login" className="btn btn-ghost btn-sm">
            Se connecter
          </Link>
        )}
      </div>
    </div>
  )
}
