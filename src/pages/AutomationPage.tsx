import { motion } from 'framer-motion';
import { systemBootContainer, systemBootItem } from '@/lib/animations';
import { MOCK_ROUTINES } from '@/lib/mockData';
import { Card } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';
import { 
  Plus, 
  Clock, 
  Terminal, 
  Server, 
  Play, 
  Edit2, 
  Trash2,
  Power
} from 'lucide-react';

export const AutomationPage = () => {
  // Helper to render the correct icon based on trigger type
  const getTriggerIcon = (type: string) => {
    switch (type) {
      case 'time': return <Clock size={18} />;
      case 'command': return <Terminal size={18} />;
      case 'device': return <Server size={18} />;
      default: return <Power size={18} />;
    }
  };

  return (
    <div className="flex flex-col gap-6 h-full">
      
      {/* Header Info & Global Actions */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
        <div>
          <h1 className="text-2xl font-mono font-bold text-primary-txt tracking-tighter">
            ROUTINE_PROTOCOLS
          </h1>
          <p className="text-secondary-txt font-mono text-xs uppercase tracking-widest flex items-center gap-2">
            <span className="w-2 h-2 rounded-full bg-jarvis-blue shadow-[0_0_8px_#00F0FF] animate-pulse" />
            {MOCK_ROUTINES.filter(r => r.isActive).length} Active Nodes
          </p>
        </div>
        
        <Button variant="primary" size="sm">
          <Plus size={14} /> Initialize Routine
        </Button>
      </div>

      {/* Routine Grid */}
      <motion.div 
        variants={systemBootContainer}
        initial="hidden"
        animate="visible"
        className="grid grid-cols-1 lg:grid-cols-2 gap-6"
      >
        {MOCK_ROUTINES.map((routine) => (
          <motion.div key={routine.id} variants={systemBootItem}>
            <Card className="group flex flex-col h-full relative overflow-visible" glow={routine.isActive}>
              
              {/* HUD Corner Accents (Top Left & Bottom Right) */}
              <div className="absolute -top-px -left-px w-4 h-4 border-t-2 border-l-2 border-jarvis-blue/50 rounded-tl-lg pointer-events-none" />
              <div className="absolute -bottom-px -right-px w-4 h-4 border-b-2 border-r-2 border-jarvis-blue/50 rounded-br-lg pointer-events-none" />

              {/* Card Header */}
              <div className="flex items-center justify-between mb-6 z-10 relative">
                <div className="flex items-center gap-3">
                  <div className={`w-2 h-2 rounded-sm ${routine.isActive ? 'bg-jarvis-blue shadow-[0_0_8px_#00F0FF]' : 'bg-surface-3'}`} />
                  <div>
                    <h3 className="text-primary-txt font-mono font-bold tracking-wide">
                      {routine.name}
                    </h3>
                    {/* Decorative Hex Code String */}
                    <p className="text-[8px] text-surface-3 font-mono tracking-widest uppercase">
                      SYS.DEF.0x{Math.floor(Math.random() * 9999).toString(16)}
                    </p>
                  </div>
                </div>
                
                {/* Custom Tech Toggle Switch */}
                <button className={`w-10 h-5 rounded-full relative transition-colors duration-300 border ${routine.isActive ? 'bg-jarvis-blue/20 border-jarvis-blue/50' : 'bg-surface-3/30 border-surface-3'}`}>
                  <motion.div 
                    initial={false}
                    animate={{ x: routine.isActive ? 20 : 2 }}
                    className={`w-4 h-4 rounded-full absolute top-0.5 shadow-lg ${routine.isActive ? 'bg-jarvis-blue shadow-[0_0_10px_#00F0FF]' : 'bg-secondary-txt'}`}
                  />
                </button>
              </div>

              {/* UPGRADED VISUAL FLOW REPRESENTATION */}
              <div className="flex-1 flex items-stretch relative bg-surface-2/40 backdrop-blur-md p-4 rounded-lg border border-surface-3 mb-6 overflow-hidden group-hover:border-surface-3/80 transition-all">
                
                {/* Decorative Tech Grid Background */}
                <div className="absolute inset-0 bg-[linear-gradient(rgba(255,255,255,0.02)_1px,transparent_1px),linear-gradient(90deg,rgba(255,255,255,0.02)_1px,transparent_1px)] bg-[size:16px_16px] pointer-events-none" />

                {/* 1. Trigger Block */}
                <div className="flex flex-col items-center justify-center text-center w-24 shrink-0 relative z-10">
                  <div className={`w-12 h-12 rounded-lg flex items-center justify-center mb-2 border backdrop-blur-md transition-all duration-500
                    ${routine.triggerType === 'time' ? 'bg-jarvis-blue/10 text-jarvis-blue border-jarvis-blue/30 shadow-[0_0_15px_rgba(0,240,255,0.1)]' : 
                      routine.triggerType === 'device' ? 'bg-error-red/10 text-error-red border-error-red/30 shadow-[0_0_15px_rgba(255,51,51,0.1)]' : 
                      'bg-success-green/10 text-success-green border-success-green/30 shadow-[0_0_15px_rgba(0,255,102,0.1)]'}
                  `}>
                    {getTriggerIcon(routine.triggerType)}
                  </div>
                  <span className="text-[9px] font-mono text-secondary-txt uppercase tracking-widest">{routine.triggerType}</span>
                  <span className="text-xs font-bold text-primary-txt mt-1 line-clamp-1" title={routine.triggerValue}>
                    {routine.triggerValue}
                  </span>
                </div>

                {/* 2. The Animated Circuit Connector */}
                <div className="flex-1 relative h-full min-w-[60px] mx-2">
                  <svg className="absolute inset-0 w-full h-full" preserveAspectRatio="none">
                    {/* Background static dashed line */}
                    <line x1="0" y1="50%" x2="100%" y2="50%" stroke="rgba(255,255,255,0.1)" strokeWidth="2" strokeDasharray="4 4" />
                    
                    {/* Animated glowing energy pulse (Only visible if active) */}
                    {routine.isActive && (
                      <motion.line 
                        x1="0" y1="50%" x2="100%" y2="50%" 
                        stroke="#00F0FF" /* Jarvis Blue */
                        strokeWidth="2"
                        initial={{ strokeDashoffset: 100 }}
                        animate={{ strokeDashoffset: 0 }}
                        transition={{ duration: 1.5, repeat: Infinity, ease: "linear" }}
                        strokeDasharray="15 85"
                        className="drop-shadow-[0_0_8px_rgba(0,240,255,0.8)]"
                      />
                    )}
                  </svg>
                </div>

                {/* 3. Action Block */}
                <div className="flex flex-col items-center justify-center text-center w-24 shrink-0 relative z-10">
                  <div className={`w-12 h-12 rounded-lg flex items-center justify-center mb-2 border backdrop-blur-md transition-all duration-500
                    ${routine.isActive ? 'bg-jarvis-blue/10 text-jarvis-blue border-jarvis-blue/50 shadow-[0_0_15px_rgba(0,240,255,0.2)]' : 'bg-surface-1 text-primary-txt border-surface-3'}
                  `}>
                    <Power size={20} />
                  </div>
                  <span className="text-[9px] font-mono text-secondary-txt uppercase tracking-widest line-clamp-1" title={routine.actionTarget}>
                    {routine.actionTarget}
                  </span>
                  <span className="text-xs font-bold text-primary-txt mt-1 line-clamp-1" title={routine.actionType}>
                    {routine.actionType}
                  </span>
                </div>

              </div>

              {/* Action Buttons Footer */}
              <div className="flex items-center justify-between pt-4 border-t border-surface-3 mt-auto relative z-10">
                <Button variant="ghost" size="sm" className="text-jarvis-blue hover:text-jarvis-blue hover:bg-jarvis-blue/10">
                  <Play size={14} /> FORCE_EXECUTE
                </Button>
                
                <div className="flex items-center gap-2">
                  <Button variant="ghost" size="sm" className="px-2 border border-surface-3 hover:border-jarvis-blue">
                    <Edit2 size={12} />
                  </Button>
                  <Button variant="ghost" size="sm" className="px-2 border border-surface-3 text-error-red hover:text-white hover:bg-error-red hover:border-error-red">
                    <Trash2 size={12} />
                  </Button>
                </div>
              </div>

            </Card>
          </motion.div>
        ))}
      </motion.div>
    </div>
  );
};