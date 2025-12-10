import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { createBrowserRouter, RouterProvider, Navigate } from 'react-router-dom'
import './index.css'
import { ThemeProvider } from './components/ThemeProvider'
import ProtectedRoute from './components/auth/ProtectedRoute'
import AppShell from './layouts/AppShell'
import LoginPage from './pages/LoginPage'
import AuthCallbackPage from './pages/AuthCallbackPage'
import AgentsIndexPage from './pages/AgentsIndexPage'
import AgentDetailPage from './pages/AgentDetailPage'
import AgentPoliciesPage from './pages/AgentPoliciesPage'

import { AuthProvider } from './contexts/AuthContext'

const router = createBrowserRouter(
  [
    {
      path: '/login',
      element: <LoginPage />,
    },
    {
      path: '/auth/callback',
      element: <AuthCallbackPage />,
    },
    {
      element: <ProtectedRoute />,
      children: [
        {
          path: '/',
          element: <Navigate to="/console/agents" replace />,
        },
        {
          path: '/console',
          element: <AppShell />,
          children: [
            { index: true, element: <Navigate to="agents" replace /> },
            { path: 'agents', element: <AgentsIndexPage /> },
            { path: 'agents/:sessionId', element: <AgentDetailPage /> },
            { path: 'agent-policies', element: <AgentPoliciesPage /> },
          ],
        },
      ],
    },
  ],
  { basename: '' }
)

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <ThemeProvider>
      <AuthProvider>
        <RouterProvider router={router} />
      </AuthProvider>
    </ThemeProvider>
  </StrictMode>
)
