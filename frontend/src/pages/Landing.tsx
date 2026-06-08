import { useState } from 'react'
import { Link, useNavigate } from 'react-router-dom'
import { z } from 'zod'
import { Navbar } from '../components/Navbar'

const loginSchema = z.object({
  username: z.string().min(1, "Nom d'utilisateur requis"),
  password: z.string().min(1, 'Mot de passe requis'),
})

type LoginForm = z.infer<typeof loginSchema>
type FieldErrors = Partial<Record<keyof LoginForm, string>>

export function Landing() {
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
    <div className="min-h-screen bg-base-200 flex flex-col">
      <Navbar />

      <div className="flex-1 flex flex-col items-center justify-center p-4 gap-8">

      {/* Brand */}
      <div className="text-center">
        <img src="/logo-default.png" alt="Cowork'in Montpellier" className="h-20 mx-auto mb-4" />
        <h1 className="text-2xl font-bold">Cowork'in Montpellier</h1>
        <p className="text-base-content/50 text-sm mt-1">Bienvenue — comment souhaitez-vous continuer ?</p>
      </div>

      {/* Two paths */}
      <div className="flex flex-col md:flex-row gap-4 w-full max-w-2xl items-stretch">

        {/* Member login */}
        <div className="card bg-base-100 shadow-md flex-1">
          <div className="card-body gap-4">
            <div>
              <h2 className="card-title text-base">Espace membres</h2>
              <p className="text-sm text-base-content/50">Connectez-vous avec votre compte coworking.</p>
            </div>

            {apiError && (
              <div role="alert" className="alert alert-error py-2 text-sm">
                <span>{apiError}</span>
              </div>
            )}

            <form onSubmit={handleSubmit} noValidate className="flex flex-col gap-3">
              <div className="form-control">
                <label className="label pb-1" htmlFor="username">
                  <span className="label-text font-medium">Identifiant</span>
                </label>
                <input
                  id="username"
                  type="text"
                  autoComplete="username"
                  autoFocus
                  placeholder="identifiant ou email"
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

              <div className="card-actions justify-between items-center mt-1">
                <Link to="/forgot-password" className="text-xs text-base-content/50 hover:text-base-content">
                  Mot de passe oublié ?
                </Link>
                <button
                  type="submit"
                  className="btn btn-primary btn-wide"
                  disabled={loading}
                >
                  {loading
                    ? <span className="loading loading-spinner loading-sm" />
                    : 'Se connecter'
                  }
                </button>
              </div>
            </form>
          </div>
        </div>

        {/* Divider */}
        <div className="flex md:flex-col items-center gap-2 text-base-content/30 text-sm font-medium">
          <div className="flex-1 border-t md:border-t-0 md:border-l border-base-300 w-full md:w-0 md:h-full" />
          <span>ou</span>
          <div className="flex-1 border-t md:border-t-0 md:border-l border-base-300 w-full md:w-0 md:h-full" />
        </div>

        {/* Guest buy */}
        <div className="card bg-base-100 shadow-md flex-1">
          <div className="card-body justify-between gap-6">
            <div>
              <h2 className="card-title text-base">Accès visiteur</h2>
              <p className="text-sm text-base-content/50">
                Achetez un accès sans créer de compte. Vos vouchers sont disponibles immédiatement après l'achat.
              </p>
            </div>
            <div className="card-actions justify-end">
              <Link to="/buy" className="btn btn-outline btn-wide">
                Acheter un accès
              </Link>
            </div>
          </div>
        </div>

      </div>

      </div>
    </div>
  )
}
