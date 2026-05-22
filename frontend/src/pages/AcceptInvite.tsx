import { useEffect, useRef, useState } from 'react'
import { useNavigate, useSearchParams } from 'react-router-dom'
import { Navbar } from '../components/Navbar'
import { acceptInvite } from '../api/invitations'
import { ApiError } from '../api/client'
import { useToast } from '../lib/toast'

export function AcceptInvite() {
  const notify = useToast()
  const navigate = useNavigate()
  const [params] = useSearchParams()
  const token = params.get('token')

  const [firstName, setFirstName] = useState('')
  const [lastName, setLastName] = useState('')
  const [username, setUsername] = useState('')
  const [password, setPassword] = useState('')
  const [confirmPassword, setConfirmPassword] = useState('')
  const [loading, setLoading] = useState(false)
  const confirmRef = useRef<HTMLInputElement>(null)

  useEffect(() => {
    if (!token) navigate('/login', { replace: true })
  }, [token, navigate])

  useEffect(() => {
    if (confirmRef.current) {
      confirmRef.current.setCustomValidity(
        confirmPassword && confirmPassword !== password ? 'no-match' : ''
      )
    }
  }, [confirmPassword, password])

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    if (!token) return
    setLoading(true)
    try {
      await acceptInvite({ token, username, first_name: firstName, last_name: lastName, password })
      notify('Compte créé avec succès. Vous pouvez vous connecter.', 'success')
      navigate('/login')
    } catch (e) {
      if (e instanceof ApiError) {
        notify(e.message, 'error')
      } else {
        notify("Erreur lors de la création du compte.", 'error')
      }
    } finally {
      setLoading(false)
    }
  }

  return (
    <div className="min-h-screen bg-base-200 flex flex-col">
      <Navbar />
      <div className="flex-1 flex items-center justify-center p-4">
        <div className="card bg-base-100 shadow-md w-full max-w-md">
          <div className="card-body gap-4">
            <h2 className="card-title text-base">Créer votre compte</h2>
            <form onSubmit={handleSubmit} className="flex flex-col gap-4">
              <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
                <div className="form-control">
                  <label className="label"><span className="label-text">Prénom</span></label>
                  <input
                    type="text"
                    className="input input-bordered w-full validator"
                    value={firstName}
                    onChange={e => setFirstName(e.target.value)}
                    required
                  />
                  <p className="validator-hint">Ce champ est requis</p>
                </div>
                <div className="form-control">
                  <label className="label"><span className="label-text">Nom</span></label>
                  <input
                    type="text"
                    className="input input-bordered w-full validator"
                    value={lastName}
                    onChange={e => setLastName(e.target.value)}
                    required
                  />
                  <p className="validator-hint">Ce champ est requis</p>
                </div>
              </div>
              <div className="form-control">
                <label className="label"><span className="label-text">Identifiant</span></label>
                <input
                  type="text"
                  className="input input-bordered w-full validator"
                  value={username}
                  onChange={e => setUsername(e.target.value)}
                  required
                />
                <p className="validator-hint">Ce champ est requis</p>
              </div>
              <div className="form-control">
                <label className="label"><span className="label-text">Mot de passe</span></label>
                <input
                  type="password"
                  className="input input-bordered w-full validator"
                  value={password}
                  onChange={e => setPassword(e.target.value)}
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
                  {loading ? <span className="loading loading-spinner loading-xs" /> : 'Créer mon compte'}
                </button>
              </div>
            </form>
          </div>
        </div>
      </div>
    </div>
  )
}
