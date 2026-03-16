import { motion } from 'framer-motion';
import { systemBootContainer, systemBootItem } from '@/lib/animations';
import { useSystemData } from '@/hooks/useSystemData'; 
import { FleetTable } from '@/components/dashboard/FleetTable';
import { GlowingChartCard } from '@/components/dashboard/GlowingChartCard';
import { Card } from '@/components/ui/Card';
import { CheckCircle2, Circle, Loader2, AlertTriangle, Globe, Zap, Activity, AlertCircle } from 'lucide-react';
import { StatCard } from '@/components/dashboard/StatCard';

export const DashboardPage = () => {
  const { stats, devices, tasks, events, history, isLoading, error } = useSystemData();

  // Handle Errors
  if (error) {
    return (
      <div className="h-full flex items-center justify-center p-6">
        <div className="text-center text-error-red flex flex-col items-center gap-4 bg-surface-1/40 backdrop-blur-md p-8 rounded-xl border border-error-red/20">
          <AlertTriangle size={48} className="animate-pulse" />
          <h2 className="text-xl font-mono font-bold tracking-widest">{error}</h2>
          <p className="text-sm text-surface-3 font-mono">CORE_UPLINK_FAILURE: 0x8842</p>
        </div>
      </div>
    );
  }

  // Handle Loading
  if (isLoading) {
    return (
      <div className="h-full flex items-center justify-center">
        <div className="flex flex-col items-center gap-4 text-jarvis-blue">
          <Loader2 size={40} className="animate-spin" />
          <p className="font-mono text-sm tracking-[0.2em] animate-pulse uppercase">Establishing Neural Uplink...</p>
        </div>
      </div>
    );
  }

  // Calculate live stats from the devices array
  const onlineDevicesCount = devices.filter(d => d.status === 'online').length;
  const totalDevicesCount = devices.length;

  return (
    <motion.div 
      variants={systemBootContainer}
      initial="hidden"
      animate="visible"
      className="flex flex-col gap-6 h-full"
    >
      {/* --- ROW 1: Quick Stats (Now Live-Linked) --- */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
        <motion.div variants={systemBootItem}>
          <StatCard icon={<Globe size={24} />} label="Network" value={stats.networkStatus} borderColorClass="border-l-jarvis-blue" iconBgClass="bg-jarvis-blue/10" iconColorClass="text-jarvis-blue" />
        </motion.div>
        
        <motion.div variants={systemBootItem}>
          <StatCard 
            icon={<Zap size={24} />} 
            label="Devices Status" 
            value={onlineDevicesCount} 
            subValue={`/ ${totalDevicesCount} Nodes`} 
            borderColorClass="border-l-success-green" 
            iconBgClass="bg-success-green/10" 
            iconColorClass="text-success-green" 
          />
        </motion.div>

        <motion.div variants={systemBootItem}>
          <StatCard icon={<Activity size={24} />} label="Active Automations" value={stats.activeAutomations} borderColorClass="border-l-primary-txt" iconBgClass="bg-white/5" iconColorClass="text-primary-txt" />
        </motion.div>
        
        <motion.div variants={systemBootItem}>
          <StatCard icon={<AlertCircle size={24} />} label="System Alerts" value="0" borderColorClass="border-l-error-red" iconBgClass="bg-error-red/10" iconColorClass="text-error-red" />
        </motion.div>
      </div>

      {/* --- ROW 2: The 3 Glowing Charts --- */}
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        <motion.div variants={systemBootItem}>
          <GlowingChartCard title="CPU USAGE" value={`${history.cpu[history.cpu.length - 1].value}%`} subValue="Avg Load" bottomLeftText="30min history" bottomRightText="8 cores active" data={history.cpu} dataKey="value" colorHex="#3b82f6" gradientId="cpuGradient" />
        </motion.div>
        <motion.div variants={systemBootItem}>
          <GlowingChartCard title="RAM USAGE" value={`${history.ram[history.ram.length - 1].value}%`} subValue="System Memory" bottomLeftText="history" bottomRightText="9.9GB / 16GB" data={history.ram} dataKey="value" colorHex="#06b6d4" gradientId="ramGradient" />
        </motion.div>
        <motion.div variants={systemBootItem}>
          <GlowingChartCard title="NET TRAFFIC" value={`${history.net[history.net.length - 1].value} Mbps`} subValue="Live Data Rate" bottomLeftText="10m  6m  now" bottomRightText="Stable" data={history.net} dataKey="value" colorHex="#d946ef" gradientId="netGradient" />
        </motion.div>
      </div>

      {/* --- ROW 3: Fleet Table (Left) + Tasks/Events (Right) --- */}
      <div className="grid grid-cols-1 xl:grid-cols-3 gap-6 flex-1">
        
        <motion.div variants={systemBootItem} className="xl:col-span-2 flex flex-col">
          <FleetTable devices={devices} />
        </motion.div>

        <div className="flex flex-col gap-6">
          <motion.div variants={systemBootItem}>
            <Card title={`CORE TASKS (${tasks.filter(t => t.status === 'active').length})`} techBg={true}>
              <div className="flex flex-col gap-3 mt-2">
                {tasks.map(task => (
                  <div key={task.id} className="flex items-center gap-3 p-2 rounded hover:bg-white/5 transition-colors">
                    {task.status === 'completed' 
                      ? <CheckCircle2 size={16} className="text-success-green" /> 
                      : <Circle size={16} className="text-jarvis-blue animate-pulse" />
                    }
                    <div className="flex-1">
                      <p className={`text-xs font-mono font-bold ${task.status === 'completed' ? 'text-surface-3 line-through' : 'text-primary-txt'}`}>
                        {task.title}
                      </p>
                      {task.status === 'active' && (
                        <div className="h-0.5 w-full bg-surface-3 mt-1.5 rounded-full overflow-hidden">
                          <motion.div 
                            initial={{ width: 0 }}
                            animate={{ width: `${task.progress}%` }}
                            className="h-full bg-jarvis-blue shadow-[0_0_8px_#00F0FF]" 
                          />
                        </div>
                      )}
                    </div>
                  </div>
                ))}
              </div>
            </Card>
          </motion.div>

          <motion.div variants={systemBootItem} className="flex-1">
            <Card title="EVENT_LOG" cornerAccents={true}>
              <div className="flex flex-col gap-4 mt-2">
                {events.map(event => (
                  <div key={event.id} className="flex gap-4 items-start border-l border-surface-3 pl-3 relative group hover:border-jarvis-blue transition-colors">
                    <div className="absolute -left-1.25 top-1 w-2 h-2 rounded-full bg-surface-3 group-hover:bg-jarvis-blue transition-colors" />
                    <span className="text-[10px] text-jarvis-blue font-mono mt-0.5 shrink-0 w-10">{event.time}</span>
                    <p className="text-xs text-primary-txt uppercase tracking-tight">{event.title}</p>
                  </div>
                ))}
              </div>
            </Card>
          </motion.div>
        </div>
      </div>
    </motion.div>
  );
};