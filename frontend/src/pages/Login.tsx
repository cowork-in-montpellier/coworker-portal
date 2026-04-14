import { useState } from 'react'
import { Link, useNavigate } from 'react-router-dom'
import { z } from 'zod'

const loginSchema = z.object({
  username: z.string().min(1, "Nom d'utilisateur requis"),
  password: z.string().min(1, 'Mot de passe requis'),
})

type LoginForm = z.infer<typeof loginSchema>
type FieldErrors = Partial<Record<keyof LoginForm, string>>

export function Login() {
  const navigate = useNavigate()
  const [form, setForm] = useState<LoginForm>({ username: '', password: '' })
  const [errors, setErrors] = useState<FieldErrors>({})
  const [apiError, setApiError] = useState<string | null>(null)
  const [loading, setLoading] = useState(false)

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()

    const result = loginSchema.safeParse(form)
    if (!result.success) {
      const fieldErrors: FieldErrors = {}
      result.error.errors.forEach(err => {
        const field = err.path[0] as keyof LoginForm
        if (!fieldErrors[field]) fieldErrors[field] = err.message
      })
      setErrors(fieldErrors)
      return
    }

    setErrors({})
    setApiError(null)
    setLoading(true)

    try {
      const res = await fetch('/api/auth/login', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(result.data),
      })

      if (res.status === 401) {
        setApiError('Identifiant ou mot de passe incorrect.')
        return
      }
      if (!res.ok) {
        setApiError('Une erreur inattendue est survenue. Veuillez réessayer.')
        return
      }

      const { token } = await res.json()
      localStorage.setItem('token', token)
      navigate('/dashboard')
    } catch {
      setApiError('Impossible de contacter le serveur. Veuillez réessayer.')
    } finally {
      setLoading(false)
    }
  }

  return (
    <div className="min-h-screen bg-base-200 flex items-center justify-center p-4">
      <div className="w-full max-w-sm">

        {/* Brand */}
        <div className="text-center mb-8">
          <div className="inline-flex items-center justify-center w-14 h-14 rounded-2xl bg-primary text-primary-content text-2xl font-bold mb-4">
            C
          </div>
          <h1 className="text-2xl font-bold text-base-content">Coworking</h1>
          <p className="text-base-content/50 text-sm mt-1">Espace membres</p>
        </div>

        {/* Card */}
        <div className="card bg-base-100 shadow-md">
          <div className="card-body gap-4">

            {apiError && (
              <div role="alert" className="alert alert-error py-2 text-sm">
                <svg xmlns="http://www.w3.org/2000/svg" className="h-4 w-4 shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
                </svg>
                <span>{apiError}</span>
              </div>
            )}

            <form onSubmit={handleSubmit} noValidate className="flex flex-col gap-4">
              <div className="form-control">
                <label className="label pb-1" htmlFor="username">
                  <span className="label-text font-medium">Identifiant</span>
                </label>
                <input
                  id="username"
                  type="text"
                  autoComplete="username"
                  autoFocus
                  placeholder="your.username"
                  className={`input input-bordered w-full ${errors.username ? 'input-error' : ''}`}
                  value={form.username}
                  onChange={e => setForm(f => ({ ...f, username: e.target.value }))}
                />
                {errors.username && (
                  <label className="label pt-1">
                    <span className="label-text-alt text-error">{errors.username}</span>
                  </label>
                )}
              </div>

              <div className="form-control">
                <label className="label pb-1" htmlFor="password">
                  <span className="label-text font-medium">Mot de passe</span>
                </label>
                <input
                  id="password"
                  type="password"
                  autoComplete="current-password"
                  placeholder="••••••••"
                  className={`input input-bordered w-full ${errors.password ? 'input-error' : ''}`}
                  value={form.password}
                  onChange={e => setForm(f => ({ ...f, password: e.target.value }))}
                />
                {errors.password && (
                  <label className="label pt-1">
                    <span className="label-text-alt text-error">{errors.password}</span>
                  </label>
                )}
              </div>

              <button
                type="submit"
                className="btn btn-primary w-full mt-2"
                disabled={loading}
              >
                {loading
                  ? <span className="loading loading-spinner loading-sm" />
                  : 'Se connecter'
                }
              </button>
            </form>

            <Link to="/buy" className="btn btn-ghost btn-sm hidden sm:flex">
              Acheter sans compte
            </Link>

          </div>
        </div>

      </div>
    </div>
  )
}
