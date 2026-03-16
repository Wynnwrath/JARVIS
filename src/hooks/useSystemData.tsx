import { useState, useEffect } from 'react';
import { 
  MOCK_SYSTEM_STATS, 
  MOCK_DEVICES, 
  MOCK_TASKS, 
  MOCK_EVENTS,
  MOCK_CPU_HISTORY,
  MOCK_RAM_HISTORY,
  MOCK_NET_HISTORY
} from '@/lib/mockData';

export const useSystemData = () => {
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  
  // Data States
  const [stats, setStats] = useState(MOCK_SYSTEM_STATS);
  const [devices, setDevices] = useState(MOCK_DEVICES);
  const [tasks, setTasks] = useState(MOCK_TASKS);
  const [events, setEvents] = useState(MOCK_EVENTS);
  const [history, setHistory] = useState({
    cpu: MOCK_CPU_HISTORY,
    ram: MOCK_RAM_HISTORY,
    net: MOCK_NET_HISTORY
  });

  const addDevice = (newDeviceData: any) => {
    const newDevice = {
      id: `nd-${Math.floor(Math.random() * 10000)}`, // Generate a random ID
      name: newDeviceData.name,
      status: 'online', // Assume it comes online when linked
      cpu: 0,        
      ram: 0,
      storage: 0,
      network: '0.0 Mbps',
      ip: newDeviceData.ip,
      mac: newDeviceData.mac
    } as typeof MOCK_DEVICES[0];
    
    // Add the new device to the beginning of the array so we see it immediately
    setDevices(prevDevices => [newDevice, ...prevDevices]);
  };

  useEffect(() => {
    const fetchBackendData = async () => {
      try {
        setIsLoading(true);
        // Simulating a 1.2 second network delay to boot up the dashboard
        await new Promise(resolve => setTimeout(resolve, 1200));
        setIsLoading(false);
      } catch (err) {
        setError("CRITICAL: Failed to establish connection with core server.");
        setIsLoading(false);
      }
    };

    fetchBackendData();
  }, []);

  return { 
    stats, 
    devices, 
    tasks, 
    events, 
    history, 
    isLoading, 
    error,
    addDevice 
  };
};