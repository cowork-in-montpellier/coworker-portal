import { BrowserRouter, Navigate, Route, Routes } from 'react-router-dom'
import { isAuthenticated } from './auth'
import { ProtectedRoute } from './components/ProtectedRoute'
import { ToastProvider } from './lib/toast'
import { AcceptInvite } from './pages/AcceptInvite'
import { CalendarPage } from './pages/Calendar'
import { CreateBill } from './pages/CreateBill'
import { Dashboard } from './pages/Dashboard'
import { ForgotPassword } from './pages/ForgotPassword'
import { GuestBuy } from './pages/GuestBuy'
import { GuestSummary } from './pages/GuestSummary'
import { InviteMember } from './pages/InviteMember'
import { Landing } from './pages/Landing'
import { Profile } from './pages/Profile'
import { ResetPassword } from './pages/ResetPassword'

function App() {
  return (
    <ToastProvider>
    <BrowserRouter>
      <Routes>
        <Route
          path="/login"
          element={isAuthenticated() ? <Navigate to="/dashboard" replace /> : <Landing />}
        />
        <Route
          path="/dashboard"
          element={<ProtectedRoute><Dashboard /></ProtectedRoute>}
        />
        <Route
          path="/bills/new"
          element={<ProtectedRoute><CreateBill /></ProtectedRoute>}
        />
        <Route path="/buy" element={<GuestBuy />} />
        <Route path="/buy/summary/:token" element={<GuestSummary />} />
        <Route path="/profile" element={<ProtectedRoute><Profile /></ProtectedRoute>} />
        <Route path="/forgot-password" element={<ForgotPassword />} />
        <Route path="/reset-password" element={<ResetPassword />} />
        <Route path="/invite" element={<ProtectedRoute><InviteMember /></ProtectedRoute>} />
        <Route path="/accept-invite" element={<AcceptInvite />} />
        <Route path="/calendar" element={<ProtectedRoute><CalendarPage /></ProtectedRoute>} />
        <Route path="*" element={<Navigate to="/dashboard" replace />} />
      </Routes>
    </BrowserRouter>
    </ToastProvider>
  )
}

export default App
