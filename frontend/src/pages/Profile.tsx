import { useEffect, useRef, useState } from 'react'
import { Link } from 'react-router-dom'
import { Navbar } from '../components/Navbar'
import { type Profile, changePassword, getProfile, updateProfile } from '../api/profile'
import { ApiError } from '../api/client'
import { useToast } from '../lib/toast'

export function Profile() {
  const notify = useToast()

  const [loading, setLoading] = useState(true)
  const [profile, setProfile] = useState<Profile | null>(null)
  const [firstName, setFirstName] = useState('')
  const [lastName, setLastName] = useState('')
  const [email, setEmail] = useState('')
  const [billingAddress, setBillingAddress] = useState('')
  const [onsitePayment, setOnsitePayment] = useState(false)
  const [savingProfile, setSavingProfile] = useState(false)

  const [currentPassword, setCurrentPassword] = useState('')
  const [newPassword, setNewPassword] = useState('')
  const [confirmPassword, setConfirmPassword] = useState('')
  const [savingPassword, setSavingPassword] = useState(false)
  const confirmRef = useRef<HTMLInputElement>(null)

  useEffect(() => {
    if (confirmRef.current) {
      confirmRef.current.setCustomValidity(
        confirmPassword && confirmPassword !== newPassword
          ? 'no-match'
          : ''
      )
    }
  }, [confirmPassword, newPassword])

  useEffect(() => {
    getProfile()
      .then(p => {
        setProfile(p)
        setFirstName(p.first_name)
        setLastName(p.last_name)
        setEmail(p.email)
        setBillingAddress(p.billing_address)
        setOnsitePayment(p.onsite_payment)
      })
      .catch(() => notify('Impossible de charger le profil.', 'error'))
      .finally(() => setLoading(false))
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])

  const handleProfileSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    setSavingProfile(true)
    try {
      const updated = await updateProfile({
        first_name: firstName,
        last_name: lastName,
        email,
        billing_address: billingAddress,
        onsite_payment: onsitePayment,
      })
      setProfile(updated)
      notify('Informations mises à jour.', 'success')
    } catch {
      notify('Erreur lors de la mise à jour.', 'error')
    } finally {
      setSavingProfile(false)
    }
  }

  const handlePasswordSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    setSavingPassword(true)
    try {
      await changePassword({ current_password: currentPassword, new_password: newPassword })
      notify('Mot de passe modifié.', 'success')
      setCurrentPassword('')
      setNewPassword('')
      setConfirmPassword('')
    } catch (e) {
      if (e instanceof ApiError && e.status === 400) {
        notify(e.message, 'error')
      } else {
        notify('Erreur lors du changement de mot de passe.', 'error')
      }
    } finally {
      setSavingPassword(false)
    }
  }

  return (
    <div className="min-h-screen bg-base-200 flex flex-col">
      <Navbar />
      <main className="flex-1 p-4 md:p-8 max-w-2xl mx-auto w-full">
        <div className="mb-6 flex items-center gap-3">
          <Link to="/dashboard" className="btn btn-ghost btn-sm">← Retour</Link>
          <h2 className="text-xl font-bold">Mon profil</h2>
        </div>

        {loading ? (
          <div className="flex justify-center py-12">
            <span className="loading loading-spinner loading-lg" />
          </div>
        ) : (
          <div className="flex flex-col gap-6">
            <div className="card bg-base-100 shadow-sm">
              <div className="card-body">
                <h3 className="card-title text-base">Informations personnelles</h3>
                {profile && (
                  <p className="text-sm text-base-content/50 mb-2 font-mono">{profile.username}</p>
                )}
                <form onSubmit={handleProfileSubmit} className="flex flex-col gap-4">
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
                    <label className="label"><span className="label-text">Email</span></label>
                    <input
                      type="email"
                      className="input input-bordered w-full validator"
                      value={email}
                      onChange={e => setEmail(e.target.value)}
                      required
                    />
                    <p className="validator-hint">Adresse email invalide</p>
                  </div>
                  <div className="form-control">
                    <label className="label"><span className="label-text">Adresse de facturation</span></label>
                    <textarea
                      className="textarea textarea-bordered w-full"
                      rows={5}
                      value={billingAddress}
                      onChange={e => setBillingAddress(e.target.value)}
                    />
                  </div>
                  <div className="form-control">
                    <label className="label cursor-pointer justify-start gap-3">
                      <input
                        type="checkbox"
                        className="toggle toggle-sm"
                        checked={onsitePayment}
                        onChange={e => setOnsitePayment(e.target.checked)}
                      />
                      <div>
                        <span className="label-text">Paiement sur place</span>
                        <p className="text-xs text-base-content/40 mt-0.5">Vous savez faire payer via le lecteur de carte SumUp une facture</p>
                      </div>
                    </label>
                  </div>
                  <div className="card-actions justify-end">
                    <button type="submit" className="btn btn-primary btn-sm" disabled={savingProfile}>
                      {savingProfile
                        ? <span className="loading loading-spinner loading-xs" />
                        : 'Enregistrer'}
                    </button>
                  </div>
                </form>
              </div>
            </div>

            <div className="card bg-base-100 shadow-sm">
              <div className="card-body">
                <h3 className="card-title text-base">Changer de mot de passe</h3>
                <form onSubmit={handlePasswordSubmit} className="flex flex-col gap-4">
                  <div className="form-control">
                    <label className="label"><span className="label-text">Mot de passe actuel</span></label>
                    <input
                      type="password"
                      className="input input-bordered w-full"
                      value={currentPassword}
                      onChange={e => setCurrentPassword(e.target.value)}
                      required
                    />
                  </div>
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
                    <label className="label"><span className="label-text">Confirmer le nouveau mot de passe</span></label>
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
                    <button type="submit" className="btn btn-primary btn-sm" disabled={savingPassword}>
                      {savingPassword
                        ? <span className="loading loading-spinner loading-xs" />
                        : 'Modifier'}
                    </button>
                  </div>
                </form>
              </div>
            </div>
          </div>
        )}
      </main>
    </div>
  )
}
