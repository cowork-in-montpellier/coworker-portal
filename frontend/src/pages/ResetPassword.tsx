import { useEffect, useRef, useState } from 'react'
import { useNavigate, useSearchParams } from 'react-router-dom'
import { Navbar } from '../components/Navbar'
import { resetPassword } from '../api/auth'
import { ApiError } from '../api/client'
import { useToast } from '../lib/toast'

export function ResetPassword() {
  const notify = useToast()
  const navigate = useNavigate()
  const [params] = useSearchParams()
  const token = params.get('token')

  const [newPassword, setNewPassword] = useState('')
  const [confirmPassword, setConfirmPassword] = useState('')
  const [loading, setLoading] = useState(false)
  const confirmRef = useRef<HTMLInputElement>(null)

  useEffect(() => {
    if (!token) {
      navigate('/login', { replace: true })
    }
  }, [token, navigate])

  useEffect(() => {
    if (confirmRef.current) {
      confirmRef.current.setCustomValidity(
        confirmPassword && confirmPassword !== newPassword ? 'no-match' : ''
      )
    }
  }, [confirmPassword, newPassword])

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    if (!token) return
    setLoading(true)
    try {
      await resetPassword(token, newPassword)
      notify('Mot de passe réinitialisé avec succès.', 'success')
      navigate('/login')
    } catch (e) {
      if (e instanceof ApiError) {
        notify(e.message, 'error')
      } else {
        notify('Erreur lors de la réinitialisation.', 'error')
      }
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
            <h2 className="card-title cw-subtitle">Nouveau mot de passe</h2>
            <form onSubmit={handleSubmit} className="flex flex-col gap-4">
              <div className="form-control">
                <label className="label"><span className="label-text">Nouveau mot de passe</span></label>
                <input
                  type="password"
                  className="input input-bordered w-full validator"
                  value={newPassword}
                  onChange={e => setNewPassword(e.target.value)}
                  required
                  minLength={8}
                  pattern="(?=.*\d)(?=.*[a-z])(?=.*[A-Z]).{8,}"
                />
                <p className="validator-hint">
                  Au moins 8 caractères avec au moins 1 chiffre, 1 minuscule et 1 majuscule
                </p>
              </div>
              <div className="form-control">
                <label className="label"><span className="label-text">Confirmer le mot de passe</span></label>
                <input
                  ref={confirmRef}
                  type="password"
                  className="input input-bordered w-full validator"
                  value={confirmPassword}
                  onChange={e => setConfirmPassword(e.target.value)}
                  required
                />
                <p className="validator-hint">Les mots de passe doivent correspondre</p>
              </div>
              <div className="card-actions justify-end">
                <button type="submit" className="btn btn-primary btn-sm" disabled={loading}>
                  {loading ? <span className="loading loading-spinner loading-xs" /> : 'Réinitialiser'}
                </button>
              </div>
            </form>
          </div>
        </div>
      </div>
    </div>
  )
}
