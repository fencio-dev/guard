import { Outlet, useLocation } from 'react-router-dom';
import { AnimatePresence, motion } from 'framer-motion';
import Sidebar from '../components/Sidebar';
import TopBar from '../components/TopBar';
import { useReducedMotion } from '../hooks/useReducedMotion';
import { Toaster } from '@/components/ui/sonner';

const AppShell = () => {
  const location = useLocation();
  const shouldReduceMotion = useReducedMotion();

  const variants = {
    initial: { opacity: 0, y: 10 },
    animate: { opacity: 1, y: 0 },
    exit: { opacity: 0, y: -10 },
  };

  return (
    <div className="flex h-screen bg-neutral-950">
      <Sidebar />
      <div className="flex-1 flex flex-col overflow-hidden">
        <TopBar />
        <main className="flex-1 overflow-y-auto bg-neutral-950">
          <div className="mx-auto max-w-7xl px-6 py-8">
            <AnimatePresence mode="wait">
              <motion.div
                key={location.pathname}
                variants={!shouldReduceMotion ? variants : undefined}
                initial="initial"
                animate="animate"
                exit="exit"
                transition={{ duration: 0.2, ease: 'easeOut' }}
              >
                <Outlet />
              </motion.div>
            </AnimatePresence>
          </div>
        </main>
      </div>
      <Toaster />
    </div>
  );
};

export default AppShell;
