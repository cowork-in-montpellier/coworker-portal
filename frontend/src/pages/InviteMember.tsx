import { useState } from 'react'
import { Link } from 'react-router-dom'
import { Navbar } from '../components/Navbar'
import { sendInvitation } from '../api/invitations'
import { ApiError } from '../api/client'
import { useToast } from '../lib/toast'

export function InviteMember() {
  const notify = useToast()
  const [email, setEmail] = useState('')
  const [loading, setLoading] = useState(false)
  const [sentTo, setSentTo] = useState<string | null>(null)

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    setLoading(true)
    try {
      await sendInvitation(email)
      setSentTo(email)
    } catch (e) {
      if (e instanceof ApiError && e.status === 409) {
        notify(e.message, 'error')
      } else {
        notify("Erreur lors de l'envoi de l'invitation.", 'error')
      }
    } finally {
      setLoading(false)
    }
  }

  return (
    <div className="min-h-screen bg-base-200 flex flex-col">
      <Navbar />
      <main className="flex-1 p-4 md:p-8 max-w-2xl mx-auto w-full">
        <div className="mb-6 flex items-center gap-3">
          <Link to="/dashboard" className="btn btn-ghost btn-sm">← Retour</Link>
          <h2 className="text-xl font-bold">Inviter un membre</h2>
        </div>

        <div className="card bg-base-100 shadow-sm">
          <div className="card-body gap-4">
            {sentTo ? (
              <div className="flex flex-col gap-4">
                <p className="text-sm text-base-content/70">
                  Une invitation a été envoyée à <span className="font-medium">{sentTo}</span>. Le lien sera valide pendant 48 heures.
                </p>
                <div className="card-actions">
                  <button className="btn btn-outline btn-sm" onClick={() => { setSentTo(null); setEmail('') }}>
                    Inviter un autre membre
                  </button>
                </div>
              </div>
            ) : (
              <>
                <p className="text-sm text-base-content/60">
                  Saisissez l'adresse email de la personne à inviter. Elle recevra un lien pour créer son compte.
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
                  <div className="card-actions justify-end">
                    <button type="submit" className="btn btn-primary btn-sm" disabled={loading}>
                      {loading ? <span className="loading loading-spinner loading-xs" /> : 'Envoyer l\'invitation'}
                    </button>
                  </div>
                </form>
              </>
            )}
          </div>
        </div>
      </main>
    </div>
  )
}
