import React from 'react';
import { Navigate, Outlet } from 'react-router-dom';
import { useAuth } from '../../contexts/AuthContext';

const ProtectedRoute: React.FC = () => {
  const { user, loading } = useAuth();
  const isDevMode = import.meta.env.VITE_DEV_MODE === 'true';

  if (loading) {
    // You can render a loading spinner here
    return <div>Loading...</div>;
  }

  // Bypass authentication in dev mode
  if (isDevMode) {
    return <Outlet />;
  }

  if (!user) {
    return <Navigate to="/login" replace />;
  }

  return <Outlet />;
};

export default ProtectedRoute;
