import { useState } from 'react';
import { Send, Radio, Search, FileText, CheckCircle, XCircle, Clock, AlertCircle, Waves } from 'lucide-react';
import { t } from '@/lib/i18n';
import { apiOrigin, basePath } from '@/lib/basePath';
import { getToken } from '@/lib/auth';

// ── Types ────────────────────────────────────────────────────────

type A2aAction = 'discover' | 'send' | 'stream' | 'status' | 'result';

interface A2aResult {
  action: A2aAction;
  success: boolean;
  output: string;
  error?: string;
  taskId?: string;
  timestamp: number;
}

function buildRpcRequest(method: string, params: unknown, id: number) {
  return {
    jsonrpc: '2.0',
    id,
    method,
    params,
  };
}

async function a2aFetch(path: string, body?: unknown, a2aToken?: string): Promise<unknown> {
  // Prefer an explicit A2A bearer token; fall back to the UI pairing token.
  const token = a2aToken || getToken();
  const headers: Record<string, string> = { 'Content-Type': 'application/json' };
  if (token) headers['Authorization'] = `Bearer ${token}`;

  const resp = await fetch(`${apiOrigin}${basePath}${path}`, {
    method: body !== undefined ? 'POST' : 'GET',
    headers,
    body: body !== undefined ? JSON.stringify(body) : undefined,
  });

  const text = await resp.text();
  if (!resp.ok) {
    throw new Error(`HTTP ${resp.status}: ${text}`);
  }
  try {
    return JSON.parse(text);
  } catch {
    return text;
  }
}

// ── Sub-components ───────────────────────────────────────────────

function StatusBadge({ success }: { success: boolean }) {
  return success ? (
    <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-[10px] font-semibold border"
      style={{ background: 'rgba(34,197,94,0.1)', borderColor: 'rgba(34,197,94,0.3)', color: '#4ade80' }}>
      <CheckCircle className="h-3 w-3" />
      {t('a2a.status_ok')}
    </span>
  ) : (
    <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-[10px] font-semibold border"
      style={{ background: 'rgba(239,68,68,0.1)', borderColor: 'rgba(239,68,68,0.3)', color: '#f87171' }}>
      <XCircle className="h-3 w-3" />
      {t('a2a.status_fail')}
    </span>
  );
}

function ResultCard({ result, onUseTaskId }: { result: A2aResult; onUseTaskId?: (id: string) => void }) {
  const actionIcon: Record<A2aAction, React.ReactNode> = {
    discover: <Search className="h-4 w-4 flex-shrink-0" style={{ color: 'var(--pc-accent)' }} />,
    send: <Send className="h-4 w-4 flex-shrink-0" style={{ color: 'var(--color-status-success)' }} />,
    stream: <Waves className="h-4 w-4 flex-shrink-0" style={{ color: '#06b6d4' }} />,
    status: <Clock className="h-4 w-4 flex-shrink-0" style={{ color: '#f59e0b' }} />,
    result: <FileText className="h-4 w-4 flex-shrink-0" style={{ color: '#a78bfa' }} />,
  };

  let formatted = result.output || result.error || '';
  try {
    formatted = JSON.stringify(JSON.parse(formatted), null, 2);
  } catch { /* leave as-is */ }

  const ts = new Date(result.timestamp).toLocaleTimeString();

  return (
    <div className="card overflow-hidden animate-slide-in-up">
      <div className="flex items-center justify-between gap-2 p-3 border-b" style={{ borderColor: 'var(--pc-border)' }}>
        <div className="flex items-center gap-2 min-w-0">
          {actionIcon[result.action]}
          <span className="text-sm font-semibold capitalize" style={{ color: 'var(--pc-text-primary)' }}>
            {result.action}
          </span>
          <StatusBadge success={result.success} />
        </div>
        <div className="flex items-center gap-2">
          {result.taskId && onUseTaskId && (
            <button
              onClick={() => onUseTaskId(result.taskId!)}
              className="text-[10px] px-2 py-1 rounded-lg border font-medium transition-colors"
              style={{ borderColor: 'var(--pc-accent-dim)', color: 'var(--pc-accent-light)', background: 'var(--pc-accent-glow)' }}
              title={t('a2a.use_task_id')}
            >
              {t('a2a.use_task_id')}
            </button>
          )}
          <span className="text-[10px] font-mono" style={{ color: 'var(--pc-text-faint)' }}>{ts}</span>
        </div>
      </div>
      {result.taskId && (
        <div className="px-3 py-1.5 text-[10px] font-mono border-b flex items-center gap-1"
          style={{ borderColor: 'var(--pc-border)', background: 'var(--pc-accent-glow)', color: 'var(--pc-accent-light)' }}>
          <span style={{ color: 'var(--pc-text-faint)' }}>{t('a2a.task_id')}:</span>
          <span>{result.taskId}</span>
        </div>
      )}
      <pre className="text-xs p-3 overflow-x-auto max-h-72 overflow-y-auto font-mono whitespace-pre-wrap break-all"
        style={{ background: 'var(--pc-bg-base)', color: result.success ? 'var(--pc-text-secondary)' : '#f87171' }}>
        {formatted || t('a2a.no_output')}
      </pre>
    </div>
  );
}

// ── Main Page ────────────────────────────────────────────────────

export default function A2aTest() {
  const [action, setAction] = useState<A2aAction>('discover');
  const [agentUrl, setAgentUrl] = useState('');
  const [bearerToken, setBearerToken] = useState('');
  const [localBearerToken, setLocalBearerToken] = useState('');
  const [message, setMessage] = useState('');
  const [taskId, setTaskId] = useState('');
  const [loading, setLoading] = useState(false);
  const [results, setResults] = useState<A2aResult[]>([]);
  const [rpcId, setRpcId] = useState(1);

  // Local A2A endpoints (same gateway server)
  const [useLocal, setUseLocal] = useState(true);
  // Use backend tool (includes pat_token and location_id from config)
  const [useBackendTool, setUseBackendTool] = useState(false);

  const actionIcons: Record<A2aAction, React.ReactNode> = {
    discover: <Search className="h-4 w-4" />,
    send: <Send className="h-4 w-4" />,
    stream: <Waves className="h-4 w-4" />,
    status: <Clock className="h-4 w-4" />,
    result: <FileText className="h-4 w-4" />,
  };

  const actions: A2aAction[] = ['discover', 'send', 'stream', 'status', 'result'];

  async function run() {
    setLoading(true);
    const ts = Date.now();
    try {
      let out: A2aResult;

      if (useLocal) {
        // Call the local gateway's A2A endpoints directly
        const tok = localBearerToken.trim() || undefined;
        if (action === 'discover') {
          const data = await a2aFetch('/.well-known/agent-card.json', undefined, tok);
          out = {
            action,
            success: true,
            output: JSON.stringify(data, null, 2),
            timestamp: ts,
          };
        } else if (useBackendTool) {
          // Use backend A2A tool with config values (pat_token, location_id)
          const reqBody = {
            action,
            url: agentUrl || undefined,
            bearer_token: localBearerToken || undefined,
            message: (action === 'send' || action === 'stream') ? message : undefined,
            task_id: (action === 'status' || action === 'result') ? taskId : undefined,
          };
          const data = await a2aFetch('/api/a2a/outbound', reqBody, tok) as Record<string, unknown>;
          const error = data.error as string | undefined;

          let extractedTaskId: string | undefined;
          let displayOutput = data.output as string || '';

          // Try to extract task_id from result (only for send, not stream)
          if (action === 'send' && data.success) {
            try {
              const parsed = JSON.parse(displayOutput);
              const result = parsed.result as Record<string, unknown> | undefined;
              if (result && typeof result.id === 'string') {
                extractedTaskId = result.id;
              }
            } catch {
              // ignore parse error
            }
          }

          out = {
            action,
            success: data.success as boolean,
            output: displayOutput,
            error: error,
            taskId: extractedTaskId,
            timestamp: ts,
          };
        } else {
          let method: string;
          let params: unknown;
          if (action === 'send') {
            method = 'message/send';
            params = {
              message: {
                role: 'user',
                parts: [{ kind: 'text', text: message }],
                messageId: crypto.randomUUID(),
              },
            };
          } else if (action === 'stream') {
            method = 'message/stream';
            params = {
              message: {
                role: 'user',
                parts: [{ kind: 'text', text: message }],
                messageId: crypto.randomUUID(),
              },
              configuration: {
                accepted_output_modes: ['text'],
              },
            };
          } else if (action === 'status' || action === 'result') {
            method = 'tasks/get';
            params = { id: taskId };
          } else {
            throw new Error('Unknown action');
          }

          const reqBody = buildRpcRequest(method, params, rpcId);
          setRpcId((n) => n + 1);
          const data = await a2aFetch('/a2a', reqBody, tok) as Record<string, unknown>;
          const result = data.result as Record<string, unknown> | undefined;
          const error = data.error as { code: number; message: string } | undefined;

          let extractedTaskId: string | undefined;
          let displayOutput = JSON.stringify(data, null, 2);

          if ((action === 'send' || action === 'stream') && result) {
            extractedTaskId = typeof result.id === 'string' ? result.id : undefined;
          }

          if (action === 'result' && result) {
            const artifacts = result.artifacts;
            if (artifacts) {
              displayOutput = JSON.stringify(artifacts, null, 2);
            }
          }

          out = {
            action,
            success: !error,
            output: displayOutput,
            error: error ? `${error.code}: ${error.message}` : undefined,
            taskId: extractedTaskId,
            timestamp: ts,
          };
        }
      } else {
        // Remote agent call via URL
        const base = agentUrl.replace(/\/$/, '');
        const token = bearerToken || undefined;
        const fetchHeaders: Record<string, string> = { 'Content-Type': 'application/json' };
        if (token) fetchHeaders['Authorization'] = `Bearer ${token}`;

        if (action === 'discover') {
          const discoverUrl = `${base}/.well-known/agent-card.json`;
          console.log('Fetching:', discoverUrl, 'with headers:', fetchHeaders);
          const resp = await fetch(discoverUrl, { headers: fetchHeaders });
          console.log('Response status:', resp.status, 'ok:', resp.ok);
          const body = await resp.text();
          console.log('Response body:', body.substring(0, 200));
          out = {
            action,
            success: resp.ok,
            output: resp.ok ? JSON.stringify(JSON.parse(body), null, 2) : '',
            error: resp.ok ? undefined : `HTTP ${resp.status}: ${body}`,
            timestamp: ts,
          };
        } else {
          let method: string;
          let params: unknown;
          if (action === 'send') {
            method = 'message/send';
            params = {
              message: {
                role: 'user',
                parts: [{ kind: 'text', text: message }],
                messageId: crypto.randomUUID(),
              },
            };
          } else if (action === 'stream') {
            method = 'message/stream';
            params = {
              message: {
                role: 'user',
                parts: [{ kind: 'text', text: message }],
                messageId: crypto.randomUUID(),
              },
              configuration: {
                accepted_output_modes: ['text'],
              },
            };
          } else {
            method = 'tasks/get';
            params = { id: taskId };
          }
          const reqBody = buildRpcRequest(method, params, rpcId);
          setRpcId((n) => n + 1);
          const resp = await fetch(`${base}/a2a`, {
            method: 'POST',
            headers: fetchHeaders,
            body: JSON.stringify(reqBody),
          });
          const bodyText = await resp.text();
          const data = resp.ok ? JSON.parse(bodyText) as Record<string, unknown> : null;
          const error = data?.error as { code: number; message: string } | undefined;
          const result = data?.result as Record<string, unknown> | undefined;

          let extractedTaskId: string | undefined;
          let displayOutput = resp.ok ? JSON.stringify(data, null, 2) : bodyText;

          if ((action === 'send' || action === 'stream') && result) {
            extractedTaskId = typeof result.id === 'string' ? result.id : undefined;
          }
          if (action === 'result' && result?.artifacts) {
            displayOutput = JSON.stringify(result.artifacts, null, 2);
          }

          out = {
            action,
            success: resp.ok && !error,
            output: displayOutput,
            error: !resp.ok ? `HTTP ${resp.status}: ${bodyText}` : error ? `${error.code}: ${error.message}` : undefined,
            taskId: extractedTaskId,
            timestamp: ts,
          };
        }
      }

      setResults((prev) => [out, ...prev]);
    } catch (err: unknown) {
      setResults((prev) => [
        {
          action,
          success: false,
          output: '',
          error: err instanceof Error ? err.message : String(err),
          timestamp: ts,
        },
        ...prev,
      ]);
    } finally {
      setLoading(false);
    }
  }

  const canRun =
    (useLocal || agentUrl.trim().length > 0) &&
    ((action === 'send' || action === 'stream') || message.trim().length > 0) &&
    (action !== 'status' && action !== 'result' || taskId.trim().length > 0 || useLocal && action === 'status');

  return (
    <div className="p-6 space-y-6 animate-fade-in">
      {/* Header */}
      <div className="flex items-center gap-3">
        <div className="p-2 rounded-2xl" style={{ background: 'var(--pc-accent-glow)', border: '1px solid var(--pc-accent-dim)' }}>
          <Radio className="h-5 w-5" style={{ color: 'var(--pc-accent)' }} />
        </div>
        <div>
          <h1 className="text-base font-semibold" style={{ color: 'var(--pc-text-primary)' }}>{t('a2a.title')}</h1>
          <p className="text-xs" style={{ color: 'var(--pc-text-muted)' }}>{t('a2a.subtitle')}</p>
        </div>
      </div>

      <div className="grid grid-cols-1 xl:grid-cols-2 gap-6">
        {/* Left: Controls */}
        <div className="space-y-4">
          {/* Mode toggle */}
          <div className="card p-4 space-y-3">
            <p className="text-xs font-semibold uppercase tracking-wider" style={{ color: 'var(--pc-text-muted)' }}>{t('a2a.target')}</p>
            <div className="flex gap-2">
              <button
                onClick={() => setUseLocal(true)}
                className="flex-1 py-2 text-xs font-medium rounded-xl border transition-all"
                style={{
                  background: useLocal ? 'var(--pc-accent-glow)' : 'transparent',
                  borderColor: useLocal ? 'var(--pc-accent-dim)' : 'var(--pc-border)',
                  color: useLocal ? 'var(--pc-accent-light)' : 'var(--pc-text-muted)',
                }}
              >
                {t('a2a.local_gateway')}
              </button>
              <button
                onClick={() => setUseLocal(false)}
                className="flex-1 py-2 text-xs font-medium rounded-xl border transition-all"
                style={{
                  background: !useLocal ? 'var(--pc-accent-glow)' : 'transparent',
                  borderColor: !useLocal ? 'var(--pc-accent-dim)' : 'var(--pc-border)',
                  color: !useLocal ? 'var(--pc-accent-light)' : 'var(--pc-text-muted)',
                }}
              >
                {t('a2a.remote_agent')}
              </button>
            </div>

            {!useLocal && (
              <div className="space-y-2 animate-fade-in">
                <input
                  type="url"
                  value={agentUrl}
                  onChange={(e) => setAgentUrl(e.target.value)}
                  placeholder={t('a2a.agent_url_placeholder')}
                  className="input-electric w-full px-3 py-2 text-sm"
                />
                <input
                  type="password"
                  value={bearerToken}
                  onChange={(e) => setBearerToken(e.target.value)}
                  placeholder={t('a2a.bearer_token_placeholder')}
                  className="input-electric w-full px-3 py-2 text-sm"
                />
              </div>
            )}

            {useLocal && (
              <div className="animate-fade-in space-y-2">
                <input
                  type="password"
                  value={localBearerToken}
                  onChange={(e) => setLocalBearerToken(e.target.value)}
                  placeholder={t('a2a.local_bearer_token_placeholder')}
                  className="input-electric w-full px-3 py-2 text-sm"
                />
                {/* Backend tool toggle */}
                <div className="flex items-center gap-2 p-2 rounded-lg border"
                  style={{ borderColor: 'var(--pc-border)', background: 'var(--pc-bg-base)' }}>
                  <input
                    type="checkbox"
                    id="useBackendTool"
                    checked={useBackendTool}
                    onChange={(e) => setUseBackendTool(e.target.checked)}
                    className="h-4 w-4 rounded"
                  />
                  <label htmlFor="useBackendTool" className="text-xs cursor-pointer"
                    style={{ color: 'var(--pc-text-secondary)' }}>
                    Use backend A2A tool (includes pat_token & location_id from config)
                  </label>
                </div>
              </div>
            )}
          </div>

          {/* Action selector */}
          <div className="card p-4 space-y-3">
            <p className="text-xs font-semibold uppercase tracking-wider" style={{ color: 'var(--pc-text-muted)' }}>{t('a2a.action')}</p>
            <div className="grid grid-cols-2 gap-2">
              {actions.map((a) => (
                <button
                  key={a}
                  onClick={() => setAction(a)}
                  className="flex items-center justify-center gap-2 py-2.5 text-xs font-medium rounded-xl border transition-all capitalize"
                  style={{
                    background: action === a ? 'var(--pc-accent-glow)' : 'transparent',
                    borderColor: action === a ? 'var(--pc-accent-dim)' : 'var(--pc-border)',
                    color: action === a ? 'var(--pc-accent-light)' : 'var(--pc-text-muted)',
                  }}
                >
                  {actionIcons[a]}
                  {a}
                </button>
              ))}
            </div>
          </div>

          {/* Params */}
          {(action === 'send' || action === 'stream') && (
            <div className="card p-4 space-y-2 animate-fade-in">
              <label className="text-xs font-semibold uppercase tracking-wider" style={{ color: 'var(--pc-text-muted)' }}>
                {t('a2a.message')}
              </label>
              <textarea
                value={message}
                onChange={(e) => setMessage(e.target.value)}
                placeholder={t('a2a.message_placeholder')}
                rows={3}
                className="input-electric w-full px-3 py-2 text-sm resize-none"
              />
            </div>
          )}

          {(action === 'status' || action === 'result') && (
            <div className="card p-4 space-y-2 animate-fade-in">
              <label className="text-xs font-semibold uppercase tracking-wider" style={{ color: 'var(--pc-text-muted)' }}>
                {t('a2a.task_id')}
              </label>
              <input
                type="text"
                value={taskId}
                onChange={(e) => setTaskId(e.target.value)}
                placeholder={t('a2a.task_id_placeholder')}
                className="input-electric w-full px-3 py-2 text-sm font-mono"
              />
              {results.some((r) => r.taskId) && (
                <p className="text-[10px]" style={{ color: 'var(--pc-text-faint)' }}>
                  {t('a2a.task_id_hint')}
                </p>
              )}
            </div>
          )}

          {/* Run button */}
          <button
            onClick={run}
            disabled={loading || !canRun}
            className="btn-electric w-full py-3 text-sm font-semibold flex items-center justify-center gap-2"
          >
            {loading ? (
              <>
                <span className="h-4 w-4 border-2 border-white/30 border-t-white rounded-full animate-spin" />
                {t('a2a.running')}
              </>
            ) : (
              <>
                <Send className="h-4 w-4" />
                {t('a2a.run')} — {action}
              </>
            )}
          </button>

          {/* Info note */}
          <div className="flex items-start gap-2 p-3 rounded-2xl border text-xs"
            style={{ background: 'rgba(59,130,246,0.06)', borderColor: 'rgba(59,130,246,0.2)', color: 'var(--pc-text-muted)' }}>
            <AlertCircle className="h-4 w-4 flex-shrink-0 mt-0.5" style={{ color: '#60a5fa' }} />
            <span>{t('a2a.info_note')}</span>
          </div>
        </div>

        {/* Right: Results */}
        <div className="space-y-4">
          <div className="flex items-center justify-between">
            <p className="text-xs font-semibold uppercase tracking-wider" style={{ color: 'var(--pc-text-muted)' }}>
              {t('a2a.results')} ({results.length})
            </p>
            {results.length > 0 && (
              <button
                onClick={() => setResults([])}
                className="text-[10px] px-2 py-1 rounded-lg border transition-colors"
                style={{ borderColor: 'var(--pc-border)', color: 'var(--pc-text-faint)' }}
              >
                {t('a2a.clear')}
              </button>
            )}
          </div>

          {results.length === 0 ? (
            <div className="card p-8 text-center">
              <Radio className="h-8 w-8 mx-auto mb-3 opacity-20" style={{ color: 'var(--pc-accent)' }} />
              <p className="text-sm" style={{ color: 'var(--pc-text-faint)' }}>{t('a2a.no_results')}</p>
            </div>
          ) : (
            <div className="space-y-3 max-h-[600px] overflow-y-auto">
              {results.map((r, i) => (
                <ResultCard
                  key={i}
                  result={r}
                  onUseTaskId={r.taskId ? (id) => { setTaskId(id); setAction('status'); } : undefined}
                />
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
