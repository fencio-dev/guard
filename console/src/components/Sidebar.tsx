import { NavLink, useLocation } from 'react-router-dom';
import { Activity, ShieldCheck, LayoutGrid } from 'lucide-react';
import { motion } from 'framer-motion';

const Sidebar = () => {
  const location = useLocation();

  const navItems = [
    { to: '/console/agents', icon: Activity, label: 'Agents' },
    { to: '/console/agent-policies', icon: ShieldCheck, label: 'Agent Policies' },
  ];

  return (
    <div className="flex flex-col h-full w-64 bg-neutral-900 border-r border-neutral-800">
      {/* Logo/Header */}
      <div className="p-6 flex items-center gap-3">
        <div className="w-10 h-10 rounded-lg bg-gradient-to-br from-white to-neutral-300 flex items-center justify-center shadow-[0_0_15px_rgba(255,255,255,0.1)]">
          <LayoutGrid className="h-5 w-5 text-black" />
        </div>
        <h1 className="text-lg font-semibold bg-gradient-hero bg-clip-text text-transparent">Console</h1>
      </div>

      {/* Navigation */}
      <nav className="flex-1 px-3 py-4 space-y-1">
        {navItems.map((item) => {
          const isActive = location.pathname === item.to;
          const Icon = item.icon;

          return (
            <NavLink
              key={item.to}
              to={item.to}
              className="relative flex items-center gap-3 px-4 py-2.5 text-sm font-medium rounded-lg transition-all duration-200"
            >
              {/* Animated active indicator background */}
              {isActive && (
                <motion.div
                  layoutId="sidebar-active"
                  className="absolute inset-0 bg-neutral-800 rounded-lg border-l-2 border-white shadow-sm"
                  initial={false}
                  transition={{
                    type: 'spring',
                    stiffness: 380,
                    damping: 30,
                  }}
                />
              )}

              {/* Content */}
              <div className={`relative z-10 flex items-center gap-3 ${
                isActive
                  ? 'text-white'
                  : 'text-neutral-400 hover:text-neutral-200'
              }`}>
                <Icon className="h-5 w-5" />
                <span>{item.label}</span>
              </div>

              {/* Hover state for non-active items */}
              {!isActive && (
                <div className="absolute inset-0 bg-neutral-800/50 rounded-lg opacity-0 hover:opacity-100 transition-opacity duration-200" />
              )}
            </NavLink>
          );
        })}
      </nav>
    </div>
  );
};

export default Sidebar;
