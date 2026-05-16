import { useEffect, useState, useRef } from 'react';
import { useParams } from 'react-router-dom';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { XIcon, CheckCircleIcon, XCircleIcon, Loader2Icon, PackageIcon } from 'lucide-react';

interface JobUpdate {
  id: string; kind: string; module: string; version: string;
  status: string; progress: number; message: string;
}

export default function InstallPage() {
  const { jobId } = useParams<{ jobId: string }>();
  const [job, setJob] = useState<JobUpdate | null>(null);
  const [logs, setLogs] = useState<string[]>([]);
  const logRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const unlisten = listen<JobUpdate>('job-update', (ev) => {
      if (ev.payload.id === jobId) {
        setJob(ev.payload);
        setLogs(prev => [...prev, ev.payload.message]);
      }
    });
    return () => { unlisten.then(fn => fn()); };
  }, [jobId]);

  useEffect(() => {
    if (logRef.current) { logRef.current.scrollTop = logRef.current.scrollHeight; }
  }, [logs]);

  const close = () => { getCurrentWindow().close(); };

  const isDone = job?.status === 'success' || job?.status === 'failed';
  const progress = job?.progress ?? 0;
  const barWidth = Math.max(2, progress * 100);

  return (
    <div className="flex flex-col h-screen bg-background" data-tauri-drag-region>
      {/* Title bar */}
      <div className="flex items-center justify-between px-4 py-3 border-b border-border shrink-0">
        <div className="flex items-center gap-2">
          <PackageIcon className="w-4 h-4 text-primary" />
          <span className="text-sm font-semibold text-foreground">
            {job ? `${job.kind} ${job.module} ${job.version}` : `Install ${jobId}`}
          </span>
        </div>
        <button onClick={close} className="p-1 rounded hover:bg-muted/30 text-muted-foreground hover:text-foreground">
          <XIcon className="w-4 h-4" />
        </button>
      </div>

      {/* Progress */}
      <div className="px-4 py-4 border-b border-border shrink-0 space-y-2">
        <div className="flex items-center justify-between text-xs">
          <span className="text-muted-foreground">
            {!job ? 'Starting...' : job.status === 'running' ? job.message : job.status === 'success' ? 'Complete' : 'Failed'}
          </span>
          <span className="font-mono text-muted-foreground">{Math.round(progress * 100)}%</span>
        </div>
        <div className="h-2 rounded-full bg-muted/30 overflow-hidden">
          <div
            className={`h-full rounded-full transition-all duration-300 ${
              job?.status === 'failed' ? 'bg-destructive' :
              job?.status === 'success' ? 'bg-success' : 'bg-primary'
            }`}
            style={{ width: `${barWidth}%` }}
          />
        </div>
      </div>

      {/* Status */}
      {job && (
        <div className="px-4 py-2 border-b border-border/50 shrink-0 flex items-center gap-2">
          {job.status === 'running' && <Loader2Icon className="w-4 h-4 text-primary animate-spin" />}
          {job.status === 'success' && <CheckCircleIcon className="w-4 h-4 text-success" />}
          {job.status === 'failed' && <XCircleIcon className="w-4 h-4 text-destructive" />}
          <span className={`text-xs font-medium ${
            job.status === 'success' ? 'text-success' :
            job.status === 'failed' ? 'text-destructive' : 'text-foreground'
          }`}>
            {job.status === 'running' && 'Installing...'}
            {job.status === 'success' && 'Installed successfully'}
            {job.status === 'failed' && 'Installation failed'}
          </span>
          {isDone && (
            <button onClick={close}
              className="ml-auto px-3 py-1 text-xs rounded-md bg-secondary hover:bg-accent border border-border text-secondary-foreground"
            >Close</button>
          )}
        </div>
      )}

      {/* Logs */}
      <div ref={logRef} className="flex-1 overflow-y-auto p-4 font-mono text-xs space-y-1 bg-background">
        {logs.map((line, i) => (
          <div key={i} className="flex items-start gap-2 text-muted-foreground">
            <span className="text-primary shrink-0">&gt;</span>
            <span className="break-all">{line}</span>
          </div>
        ))}
        {logs.length === 0 && (
          <div className="text-muted-foreground/50">Waiting for job to start...</div>
        )}
      </div>
    </div>
  );
}
