import { Card } from '@/components/ui/Card';
import { DevicesData } from '@/types'; // Adjust this path if you kept it in mockData.ts

interface FleetTableProps {
  devices: DevicesData[];
}

export const FleetTable = ({ devices }: FleetTableProps) => {
  return (
    <Card title="Fleet Status Overview" className="h-full">
      <div className="overflow-x-auto">
        <table className="w-full text-left font-mono text-sm">
          <thead>
            <tr className="text-secondary-txt border-b border-surface-3">
              <th className="pb-3 font-medium uppercase text-[10px] tracking-wider">Device Name</th>
              <th className="pb-3 font-medium uppercase text-[10px] tracking-wider text-center">CPU</th>
              <th className="pb-3 font-medium uppercase text-[10px] tracking-wider text-center">RAM</th>
              <th className="pb-3 font-medium uppercase text-[10px] tracking-wider text-center">Storage</th>
              <th className="pb-3 font-medium uppercase text-[10px] tracking-wider text-right">Net Traffic</th>
            </tr>
          </thead>
          <tbody className="divide-y divide-surface-3">
            {devices.map((device) => (
              <tr key={device.id} className="group hover:bg-white/5 transition-colors">
                <td className="py-4 flex items-center gap-3">
                  <div className={`w-2 h-2 ml-2 rounded-full ${device.status === 'online' ? 'bg-success-green shadow-[0_0_8px_#00FF66]' : 'bg-secondary-txt'}`} />
                  <span className="text-primary-txt font-bold">{device.name}</span>
                </td>
                <td className="py-4 text-center">
                  <span className={device.cpu > 80 ? "text-error-red" : "text-secondary-txt"}>{device.cpu}%</span>
                </td>
                <td className="py-4 text-center text-secondary-txt">{device.ram}%</td>
                <td className="py-4 text-center text-secondary-txt">{device.storage}%</td>
                <td className="py-4 text-right text-jarvis-blue font-bold">{device.network}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </Card>
  );
};