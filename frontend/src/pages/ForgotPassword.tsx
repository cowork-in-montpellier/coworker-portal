import { useState } from 'react'
import { Link } from 'react-router-dom'
import { Navbar } from '../components/Navbar'
import { forgotPassword } from '../api/auth'
import { useToast } from '../lib/toast'

export function ForgotPassword() {
  const notify = useToast()
  const [email, setEmail] = useState('')
  const [loading, setLoading] = useState(false)
  const [sent, setSent] = useState(false)

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    setLoading(true)
    try {
      await forgotPassword(email)
      setSent(true)
    } catch {
      notify('Erreur lors de la demande de réinitialisation.', 'error')
    } finally {
      setLoading(false)
    }
  }

  return (
    <div className="min-h-screen bg-base-200 flex flex-col">
      <Navbar />
      <div className="flex-1 flex items-center justify-center p-4">
        <div className="card cw-card w-full max-w-sm">
          <div className="card-body gap-4">
            <h2 className="card-title cw-subtitle">Mot de passe oublié</h2>

            {sent ? (
              <div className="flex flex-col gap-4">
                <p className="text-sm text-base-content/70">
                  Si cet email est enregistré, un lien de réinitialisation vous a été envoyé. Vérifiez votre boîte de réception.
                </p>
                <Link to="/login" className="btn btn-ghost btn-sm self-start">← Retour à la connexion</Link>
              </div>
            ) : (
              <>
                <p className="text-sm text-base-content/60">
                  Saisissez votre adresse email et nous vous enverrons un lien pour réinitialiser votre mot de passe.
                </p>
                <form onSubmit={handleSubmit} className="flex flex-col gap-4">
                  <div className="form-control">
                    <label className="label"><span className="label-text">Adresse email</span></label>
                    <input
                      type="email"
                      className="input input-bordered w-full validator"
                      value={email}
                      onChange={e => setEmail(e.target.value)}
                      required
                      autoFocus
                    />
                    <p className="validator-hint">Adresse email invalide</p>
                  </div>
                  <div className="card-actions justify-between items-center">
                    <Link to="/login" className="btn btn-ghost btn-sm">← Retour</Link>
                    <button type="submit" className="btn btn-primary btn-sm" disabled={loading}>
                      {loading ? <span className="loading loading-spinner loading-xs" /> : 'Envoyer'}
                    </button>
                  </div>
                </form>
              </>
            )}
          </div>
        </div>
      </div>
    </div>
  )
}
