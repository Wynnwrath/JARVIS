import React from 'react';
import { Titlebar } from '@/components/Titlebar';
import { Sidebar } from '@/components/Sidebar'; 
import { Outlet } from 'react-router-dom'; 
import { SettingsModal } from '@/components/modals/SettingsModal'

export const MainLayout = () => {
  const [isSettingsOpen, setIsSettingsOpen] = React.useState(false);

  return (
    <div className="flex h-screen w-screen bg-base text-primary-txt font-sans overflow-hidden relative">
      
      <div className="absolute top-[-10%] left-[-10%] w-[40vw] h-[40vw] rounded-full bg-jarvis-blue/20 blur-[120px] pointer-events-none z-0" />
      <div className="absolute bottom-[-10%] right-[-10%] w-[30vw] h-[30vw] rounded-full bg-success-green/10 blur-[100px] pointer-events-none z-0" />

      <div className="z-10 h-full flex shrink-0">
        <Sidebar onSettingsClick={() => setIsSettingsOpen(true)} />
      </div>
      
      <div className="flex flex-col flex-1 overflow-hidden z-10 relative">
        
        <Titlebar />
        
        <main className="flex-1 overflow-y-auto p-6 custom-scrollbar">
          <div className="max-w-400 mx-auto h-full">
            <Outlet />
          </div>
        </main>

      </div>

      <SettingsModal 
        isOpen={isSettingsOpen} 
        onClose={() => setIsSettingsOpen(false)} 
      />

    </div>
  );
};