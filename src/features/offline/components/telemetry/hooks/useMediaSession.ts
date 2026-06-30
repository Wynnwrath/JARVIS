import { useState, useEffect, useRef } from 'react';
import {
  mediaControls,
  PlaybackStatus,
  getMetadata,
  getPlaybackInfo,
  isEnabled,
  getPlaybackStatus,
  getPosition,
  type MediaMetadata,
  type PlaybackInfo,
} from 'tauri-plugin-media-api';

interface UseMediaSessionReturn {
  isPlaying: boolean;
  trackTitle: string;
  trackArtist: string;
  trackProgress: number;
  trackDuration: number;
  coverArt: string | null;
  isMediaSupported: boolean;
  hasActiveMedia: boolean;
  mediaSource: string;
  togglePlayPause: () => void;
}

export const useMediaSession = (): UseMediaSessionReturn => {
  const [isPlaying, setIsPlaying] = useState(false);
  const [trackTitle, setTrackTitle] = useState('Station Calibration');
  const [trackArtist, setTrackArtist] = useState('Neural Network');
  const [trackProgress, setTrackProgress] = useState(105);
  const [trackDuration, setTrackDuration] = useState(200);
  const [mediaSource, setMediaSource] = useState('Spotify');
  const [isMediaSupported, setIsMediaSupported] = useState(false);
  const [hasActiveMedia, setHasActiveMedia] = useState(false);
  const [coverArt, setCoverArt] = useState<string | null>(null);
  const lastPosFromOsRef = useRef(0);
  const localPosOffsetRef = useRef(0);
  const lastTrackTitleRef = useRef<string | null>(null);

  useEffect(() => {
    if (lastTrackTitleRef.current !== null && lastTrackTitleRef.current !== trackTitle) {
      lastPosFromOsRef.current = 0;
      localPosOffsetRef.current = 0;
      setTrackProgress(0);
    }
    lastTrackTitleRef.current = trackTitle;
  }, [trackTitle]);

  useEffect(() => {
    let active = true;
    const initAndCheckSupport = async () => {
      try {
        await mediaControls.initialize('jarvis', 'JARVIS Media');
        const supported = await isEnabled();
        if (active) setIsMediaSupported(supported);
      } catch (err: unknown) {
        console.warn('Media controls not supported or outside Tauri environment:', err);
        if (active) setIsMediaSupported(false);
      }
    };
    initAndCheckSupport();
    return () => { active = false; };
  }, []);

  useEffect(() => {
    let active = true;

    if (isMediaSupported) {
      const interval = setInterval(async () => {
        try {
          let metadata: MediaMetadata | null = null;
          try {
            metadata = await getMetadata();
          } catch (e: unknown) { console.warn('getMetadata failed:', e); }

          let info: PlaybackInfo | null = null;
          try {
            info = await getPlaybackInfo();
          } catch (e: unknown) {
            const errMsg = String(e);
            if (!errMsg.includes('0x00000000') && !errMsg.includes('completed successfully')) {
              console.warn('getPlaybackInfo failed:', e);
            }
          }

          if (!active) return;

          let statusVal: PlaybackStatus = PlaybackStatus.Stopped;
          let posVal = 0;
          if (info) {
            statusVal = info.status;
            posVal = info.position ?? 0;
          } else {
            try { statusVal = await getPlaybackStatus(); } catch { /* fallback */ }
            try { posVal = await getPosition(); } catch { /* fallback */ }
          }

          if (metadata && (metadata.title || metadata.artist)) {
            setHasActiveMedia(true);
            setTrackTitle(metadata.title ?? 'Unknown Title');
            setTrackArtist(metadata.artist ?? 'Unknown Artist');

            if (metadata.duration && metadata.duration > 0) {
              setTrackDuration(metadata.duration);
            } else {
              setTrackDuration(0);
            }

            setIsPlaying(statusVal === PlaybackStatus.Playing);

            if (posVal > 0) {
              lastPosFromOsRef.current = posVal;
              localPosOffsetRef.current = 0;
              setTrackProgress(posVal);
            } else if (statusVal === PlaybackStatus.Playing) {
              localPosOffsetRef.current += 1;
              setTrackProgress(lastPosFromOsRef.current + localPosOffsetRef.current);
            } else {
              localPosOffsetRef.current = 0;
              setTrackProgress(lastPosFromOsRef.current);
            }

            if (metadata.artworkData) {
              const src = metadata.artworkData.startsWith('data:')
                ? metadata.artworkData
                : `data:image/png;base64,${metadata.artworkData}`;
              setCoverArt(src);
            } else if (metadata.artworkUrl) {
              setCoverArt(metadata.artworkUrl);
            } else {
              setCoverArt(null);
            }

            if (metadata.albumArtist) {
              setMediaSource(metadata.albumArtist);
            } else if (metadata.album) {
              setMediaSource(metadata.album);
            } else {
              setMediaSource('System Player');
            }
          } else {
            setHasActiveMedia(false);
            setIsPlaying(false);
            setCoverArt(null);
          }
        } catch (err: unknown) {
          const errMsg = String(err);
          if (!errMsg.includes('0x00000000') && !errMsg.includes('completed successfully')) {
            console.warn('Media polling error:', err);
          }
        }
      }, 1000);
      return () => { active = false; clearInterval(interval); };
    }

    const fallbackInterval = setInterval(() => {
      setHasActiveMedia(true);
      setTrackProgress(prev => {
        if (isPlaying) return prev >= trackDuration ? 0 : prev + 1;
        return prev;
      });
    }, 1000);
    return () => { active = false; clearInterval(fallbackInterval); };
  }, [isMediaSupported, isPlaying, trackDuration]);

  const togglePlayPause = async () => {
    try {
      await mediaControls.togglePlayPause();
      let newPlaying = !isPlaying;
      try {
        const info = await getPlaybackInfo();
        if (info) {
          newPlaying = info.status === PlaybackStatus.Playing;
        } else {
          const status = await getPlaybackStatus();
          newPlaying = status === PlaybackStatus.Playing;
        }
      } catch {
        try {
          const status = await getPlaybackStatus();
          newPlaying = status === PlaybackStatus.Playing;
        } catch { /* keep optimistic flip */ }
      }
      setIsPlaying(newPlaying);
    } catch (err: unknown) {
      const errMsg = String(err);
      if (!errMsg.includes('0x00000000') && !errMsg.includes('completed successfully')) {
        console.warn('Toggle play/pause failed:', err);
      }
      setIsPlaying(!isPlaying);
    }
  };

  return {
    isPlaying, trackTitle, trackArtist, trackProgress, trackDuration,
    coverArt, isMediaSupported, hasActiveMedia, mediaSource, togglePlayPause,
  };
};
