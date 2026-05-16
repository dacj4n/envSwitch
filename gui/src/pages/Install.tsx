import { useEffect, useState, useRef } from 'react';
import { useParams } from 'react-router-dom';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { XIcon, CheckCircleIcon, XCircleIcon, Loader2Icon, PackageIcon, DownloadIcon, FileArchiveIcon, ShieldCheckIcon, WrenchIcon } from 'lucide-react';

interface JobUpdate {
  id: string; kind: string; module: string; version: string;
  status: string; progress: number; message: string;
  phase: string; downloaded_bytes: number; total_bytes: number;
  speed_bytes: number; eta_seconds: number;
}

const PHASES = [
  { key: 'fetch', label: 'Fetching', icon: DownloadIcon },
  { key: 'download', label: 'Downloading', icon: DownloadIcon },
  { key: 'verify', label: 'Verifying', icon: ShieldCheckIcon },
  { key: 'extract', label: 'Extracting', icon: FileArchiveIcon },
  { key: 'install', label: 'Installing', icon: WrenchIcon },
  { key: 'done', label: 'Complete', icon: CheckCircleIcon },
  { key: 'error', label: 'Error', icon: XCircleIcon },
];

function fmtBytes(b: number) {
  if (b === 0) return '';
  if (b > 1024 * 1024) return `${(b / 1024 / 1024).toFixed(1)} MB`;
  if (b > 1024) return `${(b / 1024).toFixed(1)} KB`;
  return `${b} B`;
}

function fmtSpeed(bps: number) {
  if (bps === 0) return '';
  return `${fmtBytes(bps)}/s`;
}

function fmtETA(sec: number) {
  if (sec === 0) return '';
  if (sec > 60) return `${Math.ceil(sec / 60)}m`;
  return `${sec}s`;
}

export default function InstallPage() {
  const { jobId } = useParams<{ jobId: string }>();
  const [job, setJob] = useState<JobUpdate | null>(null);
  const [logs, setLogs] = useState<string[]>([]);
  const logRef = useRef<HTMLDivElement>(null);
  const prevPhase = useRef('');

  useEffect(() => {
    const unlisten = listen<JobUpdate>('job-update', (ev) => {
      if (ev.payload.id === jobId) {
        setJob(ev.payload);
        if (ev.payload.phase !== prevPhase.current) {
          prevPhase.current = ev.payload.phase;
          setLogs(prev => [...prev, `[${ev.payload.phase}] ${ev.payload.message}`]);
        }
      }
    });
    return () => { unlisten.then(fn => fn()); };
  }, [jobId]);

  useEffect(() => {
    if (logRef.current) { logRef.current.scrollTop = logRef.current.scrollHeight; }
  }, [logs]);

  // Animate progress: during download/extract, auto-increment slowly toward target
  const [animProgress, setAnimProgress] = useState(0);
  const targetRef = useRef(0);
  const tickRef = useRef(0);
  useEffect(() => {
    targetRef.current = job?.progress ?? 0;
    if (job?.status === 'success') { setAnimProgress(1); return; }
    if (job?.status === 'failed') { return; }
    tickRef.current = 0;
    const timer = setInterval(() => {
      tickRef.current += 1;
      setAnimProgress(prev => {
        const target = targetRef.current;
        // Auto-advance slowly (0.3% per tick) during download/extract phases, capped at 95% of target
        const phaseStep = (job?.phase === 'download' || job?.phase === 'extract') ? 0.003 : 0;
        const auto = Math.min(prev + phaseStep, target * 0.95);
        if (auto > prev) return auto;
        if (prev >= target) return prev;
        const next = prev + (target - prev) * 0.08;
        return next >= target ? target : next;
      });
    }, 100);
    return () => clearInterval(timer);
  }, [job?.progress, job?.status, job?.phase]);

  const close = () => { getCurrentWindow().close(); };
  const isDone = job?.status === 'success' || job?.status === 'failed';
  const progress = animProgress;
  const curPhase = PHASES.find(p => p.key === job?.phase);

  return (
    <div className="flex flex-col h-screen bg-background" data-tauri-drag-region>
      {/* Title */}
      <div className="flex items-center justify-between px-5 py-4 border-b border-border shrink-0">
        <div className="flex items-center gap-3">
          <div className="w-9 h-9 rounded-lg flex items-center justify-center"
            style={{ backgroundColor: '#22c55e18', border: '1px solid #22c55e30' }}>
            <PackageIcon className="w-4 h-4 text-primary" />
          </div>
          <div>
            <div className="font-semibold text-sm text-foreground">
              {job ? `${job.kind} ${job.module} ${job.version}` : 'Installing...'}
            </div>
            <div className="text-xs text-muted-foreground mt-0.5">{job?.message || 'Starting...'}</div>
          </div>
        </div>
        <button onClick={close} className="p-1 rounded hover:bg-muted/30 text-muted-foreground">
          <XIcon className="w-4 h-4" />
        </button>
      </div>

      {/* Phase steps */}
      <div className="px-5 py-3 border-b border-border shrink-0 flex items-center gap-1">
        {PHASES.filter(p => p.key !== 'done' && p.key !== 'error').map((p, i) => {
          const done = job?.phase === 'done' || (job?.phase && PHASES.findIndex(x => x.key === job.phase) > PHASES.findIndex(x => x.key === p.key));
          const active = job?.phase === p.key;
          const failed = job?.phase === 'error' && PHASES.findIndex(x => x.key === 'error') - 1 === i;
          return (
            <div key={p.key} className="flex items-center gap-1">
              <div className={`flex items-center gap-1 px-2 py-1 rounded text-[10px] font-medium ${
                failed ? 'bg-destructive/10 text-destructive border border-destructive/30' :
                done ? 'bg-success/10 text-success border border-success/30' :
                active ? 'bg-primary/10 text-primary border border-primary/30' :
                'bg-muted/20 text-muted-foreground border border-border/50'
              }`}>
                {active ? <Loader2Icon className="w-2.5 h-2.5 animate-spin" /> :
                 done ? <CheckCircleIcon className="w-2.5 h-2.5" /> :
                 failed ? <XCircleIcon className="w-2.5 h-2.5" /> :
                 <div className="w-2.5 h-2.5 rounded-full bg-muted-foreground/30" />}
                {p.label}
              </div>
              {i < 4 && <div className={`w-3 h-px ${done ? 'bg-success/30' : 'bg-border'}`} />}
            </div>
          );
        })}
      </div>

      {/* Progress + stats */}
      <div className="px-5 py-3 border-b border-border shrink-0 space-y-2">
        <div className="flex items-center justify-between text-xs">
          <span className="text-muted-foreground font-mono">{Math.round(progress * 100)}%</span>
          <div className="flex items-center gap-4 text-[11px] text-muted-foreground font-mono">
            {job && job.total_bytes > 0 && <span>{fmtBytes(job.downloaded_bytes)} / {fmtBytes(job.total_bytes)}</span>}
            {job && job.speed_bytes > 0 && <span>{fmtSpeed(job.speed_bytes)}</span>}
            {job && job.eta_seconds > 0 && <span>ETA {fmtETA(job.eta_seconds)}</span>}
          </div>
        </div>
        <div className="h-2 rounded-full bg-muted/30 overflow-hidden">
          <div className={`h-full rounded-full transition-all duration-500 ${
            job?.status === 'failed' ? 'bg-destructive' :
            job?.status === 'success' ? 'bg-success' : 'bg-primary'
          }`} style={{ width: `${Math.max(2, progress * 100)}%` }} />
        </div>
      </div>

      {/* Status line */}
      {job && (
        <div className="px-5 py-2 border-b border-border/50 shrink-0 flex items-center gap-2">
          {job.status === 'running' && <Loader2Icon className="w-4 h-4 text-primary animate-spin" />}
          {job.status === 'success' && <CheckCircleIcon className="w-4 h-4 text-success" />}
          {job.status === 'failed' && <XCircleIcon className="w-4 h-4 text-destructive" />}
          <span className={`text-xs font-medium ${job.status === 'success' ? 'text-success' : job.status === 'failed' ? 'text-destructive' : 'text-foreground'}`}>
            {job.status === 'running' && curPhase?.label + '...'}
            {job.status === 'success' && 'Installation complete'}
            {job.status === 'failed' && 'Installation failed'}
          </span>
          {isDone ? (
            <button onClick={close} className="ml-auto px-3 py-1 text-xs rounded-md bg-secondary hover:bg-accent border border-border">Close</button>
          ) : (
            <button onClick={() => { invoke('cancel_job', { jobId }); }}
              className="ml-auto px-3 py-1 text-xs rounded-md bg-destructive/10 text-destructive hover:bg-destructive/20 border border-destructive/30"
            >Cancel</button>
          )}
        </div>
      )}

      {/* Logs */}
      <div ref={logRef} className="flex-1 overflow-y-auto p-4 font-mono text-xs space-y-0.5 bg-[#0a0d14]">
        {logs.map((line, i) => (
          <div key={i} className="flex items-start gap-2">
            <span className="text-primary/60 shrink-0 select-none">&gt;</span>
            <span className={`break-all ${line.includes('Error') || line.includes('failed') ? 'text-destructive' : 'text-muted-foreground'}`}>{line}</span>
          </div>
        ))}
        {logs.length === 0 && <div className="text-muted-foreground/30 italic">Waiting for task...</div>}
      </div>
    </div>
  );
}
