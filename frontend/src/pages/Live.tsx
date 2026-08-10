import { useEffect, useState } from 'react'
import { type ConnectedGuestsResponse, fetchConnected } from '../api/connected'
import { Navbar } from '../components/Navbar'

export function Live() {
  const [connected, setConnected] = useState<ConnectedGuestsResponse | null>(null)
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    fetchConnected()
      .then(setConnected)
      .catch(() => { /* non-fatal */ })
      .finally(() => setLoading(false))
  }, [])

  return (
    <div className="min-h-screen bg-base-200 flex flex-col">
      <Navbar />

      <main className="flex-1 p-4 md:p-8 max-w-6xl mx-auto w-full">
        <h2 className="text-xl font-bold mb-6">Le cowo en temps réel</h2>

        <div className="card bg-base-100 shadow-sm">
          <div className="card-body p-4">
            <div className="flex items-center justify-between mb-3">
              <h3 className="font-semibold text-sm">Connectés en ce moment</h3>
              {loading && <span className="loading loading-spinner loading-xs text-base-content/40" />}
            </div>

            {!loading && connected === null && (
              <p className="text-sm text-base-content/40">Impossible de charger les données.</p>
            )}

            {!loading && connected !== null && connected.total === 0 && (
              <p className="text-sm text-base-content/40">Aucun utilisateur connecté actuellement.</p>
            )}

            {!loading && connected !== null && connected.total > 0 && (
              <div className="flex flex-wrap gap-4">
                <div className="flex items-center gap-2">
                  <span className="text-2xl font-bold">{connected.total}</span>
                  <span className="text-sm text-base-content/60">connecté{connected.total !== 1 ? 's' : ''}</span>
                </div>
                <div className="flex items-center gap-3 text-sm text-base-content/60">
                  <span>
                    <span className="font-medium text-base-content">{connected.account_users.length}</span>
                    {' '}membre{connected.account_users.length !== 1 ? 's' : ''}
                  </span>
                  <span>·</span>
                  <span>
                    <span className="font-medium text-base-content">{connected.guest_count}</span>
                    {' '}invité{connected.guest_count !== 1 ? 's' : ''}
                  </span>
                  {connected.unknown_count > 0 && (
                    <>
                      <span>·</span>
                      <span className="text-base-content/40">
                        {connected.unknown_count} essai{connected.unknown_count !== 1 ? 's' : ''}
                      </span>
                    </>
                  )}
                </div>
              </div>
            )}
          </div>
        </div>
      </main>
    </div>
  )
}
