import { motion } from 'framer-motion';

interface AudioVisualizerProps {
  barCount?: number;
  isPlaying?: boolean;
}

export const AudioVisualizer = ({ barCount = 6, isPlaying = true }: AudioVisualizerProps) => (
  <div className="flex items-end gap-[2px] h-8">
    {Array.from({ length: barCount }).map((_, idx) => (
      <motion.div
        key={idx}
        animate={
          isPlaying
            ? { height: [4, 12 + Math.random() * 16, 8, 18 + Math.random() * 12, 4].map(h => `${h}px`) }
            : { height: '4px' }
        }
        transition={{
          duration: 0.6 + idx * 0.15,
          repeat: Infinity,
          ease: 'easeInOut',
        }}
        className="w-[3px] bg-offline-core/80 rounded-t"
      />
    ))}
  </div>
);
