import { BrowserRouter, Navigate, Route, Routes } from 'react-router-dom'
import { isAuthenticated } from './auth'
import { ProtectedRoute } from './components/ProtectedRoute'
import { CreateBill } from './pages/CreateBill'
import { Dashboard } from './pages/Dashboard'
import { GuestBuy } from './pages/GuestBuy'
import { GuestSummary } from './pages/GuestSummary'
import { Login } from './pages/Login'

function App() {
  return (
    <BrowserRouter>
      <Routes>
        <Route
          path="/login"
          element={isAuthenticated() ? <Navigate to="/dashboard" replace /> : <Login />}
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
        <Route path="*" element={<Navigate to="/dashboard" replace />} />
      </Routes>
    </BrowserRouter>
  )
}

export default App
