import { motion } from 'framer-motion';

interface MetricBarProps {
  icon: React.ReactNode;
  label: string;
  value: number;
  isWarning?: boolean;
  baseColorClass?: string;
}

export const MetricBar = ({ 
  icon, 
  label, 
  value, 
  isWarning = false,
  baseColorClass = "bg-jarvis-blue text-jarvis-blue"
}: MetricBarProps) => {
  const textColor = isWarning ? "text-error-red" : baseColorClass.split(' ')[1];
  const bgColor = isWarning ? "bg-error-red" : baseColorClass.split(' ')[0];

  return (
    <div className="flex flex-col gap-1.5">
      <div className="flex justify-between font-mono text-[10px] uppercase tracking-wider">
        <span className={`flex items-center gap-1.5 ${textColor}`}>
          {icon} <span className="text-secondary-txt">{label}</span>
        </span>
        <span className={isWarning ? "text-error-red font-bold" : "text-primary-txt"}>
          {value}%
        </span>
      </div>
      <div className="h-1 w-full bg-surface-3 rounded-full overflow-hidden">
        <motion.div 
          initial={{ width: 0 }}
          animate={{ width: `${value}%` }}
          className={`h-full ${bgColor}`}
        />
      </div>
    </div>
  );
};