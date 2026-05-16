import { useEffect, useState, useRef } from 'react';
import { useParams } from 'react-router-dom';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { XIcon, CheckCircleIcon, XCircleIcon, Loader2Icon, PackageIcon } from 'lucide-react';

interface JobUpdate {
  id: string; kind: string; module: string; version: string;
  status: string; progress: number; message: string; phase: string;
}

const PHASES = ['fetching', 'downloading', 'verifying', 'extracting', 'installing', 'done'];

export default function InstallPage() {
  const { jobId } = useParams<{ jobId: string }>();
  const [job, setJob] = useState<JobUpdate | null>(null);
  const [logs, setLogs] = useState<string[]>([]);
  const logRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    // Poll current state immediately (backend may have started before window opened)
    invoke<JobUpdate | null>('get_job_state', { jobId }).then(j => {
      if (j) { setJob(j); setLogs([`[${j.phase}] ${j.message}`]); }
    });

    const unlisten = listen<JobUpdate>('job-update', (ev) => {
      if (ev.payload.id !== jobId) return;
      setJob(ev.payload);
      setLogs(prev => {
        const msg = `[${ev.payload.phase}] ${ev.payload.message}`;
        if (prev[prev.length - 1] === msg) return prev;
        return [...prev, msg];
      });
    });
    return () => { unlisten.then(fn => fn()); };
  }, [jobId]);

  useEffect(() => {
    if (logRef.current) logRef.current.scrollTop = logRef.current.scrollHeight;
  }, [logs]);

  const close = () => getCurrentWindow().close();
  const cancel = () => { invoke('cancel_job', { jobId }); };
  const done = job?.status === 'success' || job?.status === 'failed' || job?.status === 'cancelled';
  const pct = Math.round((job?.progress ?? 0) * 100);
  const curIdx = PHASES.indexOf(job?.phase ?? 'fetch');
  const failed = job?.status === 'failed';

  return (
    <div className="flex flex-col h-screen bg-[#0a0d14]" data-tauri-drag-region>
      {/* Title bar */}
      <div className="flex items-center justify-between px-5 py-4 border-b border-[#1a2030] shrink-0">
        <div className="flex items-center gap-3">
          <div className="w-8 h-8 rounded-lg flex items-center justify-center bg-primary/10 border border-primary/20">
            <PackageIcon className="w-4 h-4 text-primary" />
          </div>
          <div>
            <div className="text-sm font-semibold text-foreground">
              {job ? `${job.module} ${job.version}` : 'Installing...'}
            </div>
            <div className="text-[11px] text-muted-foreground mt-0.5 font-mono">
              {job?.status === 'running' ? job?.message || 'Running...' :
               job?.status === 'success' ? 'Complete' :
               job?.status === 'failed' ? 'Failed' :
               job?.status === 'cancelled' ? 'Cancelled' : 'Starting...'}
            </div>
          </div>
        </div>
        <div className="flex items-center gap-2">
          {!done && (
            <button onClick={cancel}
              className="px-2.5 py-1 text-[11px] rounded-md bg-destructive/10 text-destructive hover:bg-destructive/20 border border-destructive/20 font-medium"
            >Cancel</button>
          )}
          <button onClick={close} className="p-1 rounded hover:bg-white/5 text-muted-foreground">
            <XIcon className="w-4 h-4" />
          </button>
        </div>
      </div>

      {/* Status + progress */}
      <div className="px-5 py-4 border-b border-[#1a2030] shrink-0 space-y-3">
        <div className="flex items-center gap-2">
          {!job && <Loader2Icon className="w-4 h-4 text-primary animate-spin" />}
          {job?.status === 'running' && <Loader2Icon className="w-4 h-4 text-primary animate-spin" />}
          {job?.status === 'success' && <CheckCircleIcon className="w-4 h-4 text-success" />}
          {failed && <XCircleIcon className="w-4 h-4 text-destructive" />}
          {job?.status === 'cancelled' && <XCircleIcon className="w-4 h-4 text-warning" />}
          <span className={`text-xs font-medium ${
            job?.status === 'success' ? 'text-success' : failed ? 'text-destructive' : 'text-foreground'
          }`}>
            {!job ? 'Initializing...' :
             job.status === 'running' ? 'Running' :
             job.status === 'success' ? 'Installed successfully' :
             job.status === 'failed' ? 'Installation failed' :
             job.status === 'cancelled' ? 'Cancelled' : job.status}
          </span>
          <span className="ml-auto font-mono text-xs text-muted-foreground">{pct}%</span>
        </div>

        {/* Progress bar */}
        <div className="h-1.5 rounded-full bg-white/5 overflow-hidden">
          <div className={`h-full rounded-full transition-all duration-700 ${
            failed || job?.status === 'cancelled' ? 'bg-destructive' :
            done ? 'bg-success' : 'bg-primary'
          }`} style={{ width: `${Math.max(2, pct)}%` }} />
        </div>

        {/* Phase steps */}
        <div className="flex items-center gap-1">
          {PHASES.filter(p => p !== 'done').map((p, i) => {
            const past = curIdx > i || done;
            const active = curIdx === i && !done;
            return (
              <div key={p} className="flex items-center gap-1">
                <div className={`flex items-center gap-1 px-2 py-0.5 rounded text-[9px] font-medium ${
                  failed && i === curIdx ? 'bg-destructive/15 text-destructive border border-destructive/30' :
                  past ? 'bg-success/10 text-success/70 border border-success/20' :
                  active ? 'bg-primary/15 text-primary border border-primary/30' :
                  'bg-white/3 text-muted-foreground/50 border border-white/5'
                }`}>
                  {active && !done && <Loader2Icon className="w-2 h-2 animate-spin" />}
                  {past && <CheckCircleIcon className="w-2 h-2" />}
                  {!past && !active && <div className="w-2 h-2 rounded-full bg-white/10" />}
                  {p}
                </div>
                {i < 4 && <div className={`w-2 h-px ${past ? 'bg-success/30' : 'bg-white/5'}`} />}
              </div>
            );
          })}
        </div>
      </div>

      {/* Logs */}
      <div ref={logRef} className="flex-1 overflow-y-auto p-4 font-mono text-[11px] space-y-0.5">
        {logs.map((line, i) => {
          const isError = line.includes('Error') || line.includes('failed');
          const isPhase = line.startsWith('[');
          return (
            <div key={i} className={`flex items-start gap-2 ${isError ? 'text-destructive' : isPhase ? 'text-primary/80' : 'text-muted-foreground'}`}>
              <span className="text-primary/40 shrink-0 select-none">&gt;</span>
              <span className="break-all">{line}</span>
            </div>
          );
        })}
        {logs.length === 0 && (
          <div className="text-muted-foreground/20 italic">Waiting for job to begin...</div>
        )}
      </div>
    </div>
  );
}
