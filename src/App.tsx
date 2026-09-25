import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { Copy, Moon, Sun, X } from '@phosphor-icons/react'
import { useCalculator } from './useCalculator'
import { KEYBOARD_MAP, MAIN_ROWS, SCI_ROWS, type KeyDef } from './keys'

const OP_CHAR_TO_ID: Record<string, string> = {
  '+': 'add',
  '-': 'sub',
  '*': 'mul',
  '/': 'div',
  '^': 'pow',
  r: 'root',
  E: 'ee',
}

function displaySize(len: number): number {
  if (len <= 6) return 84
  return Math.max(36, 84 - (len - 6) * 7)
}

interface RunState {
  label: string
  pct: number
  resumed: boolean
  formatting: boolean
}

function CalcKey({
  def,
  active,
  flash,
  onPress,
}: {
  def: KeyDef
  active: boolean
  flash: boolean
  onPress: (id: string) => void
}) {
  return (
    <button
      type="button"
      className={`key key-${def.kind}${def.wide ? ' key-wide' : ''}${active ? ' is-active' : ''}${flash ? ' is-flash' : ''}`}
      onClick={() => onPress(def.id)}
      aria-label={def.label}
    >
      {def.label}
    </button>
  )
}

export default function App() {
  const { ready, snap, press, setBigResult, fullResult } = useCalculator()
  const [theme, setTheme] = useState<'dark' | 'light'>('dark')
  const [flash, setFlash] = useState<string | null>(null)
  const [run, setRun] = useState<RunState | null>(null)
  const [copied, setCopied] = useState(false)
  const [jobErr, setJobErr] = useState<string | null>(null)
  const tapeRef = useRef<HTMLDivElement>(null)
  const flashTimer = useRef<number | undefined>(undefined)
  const copiedTimer = useRef<number | undefined>(undefined)
  const workerRef = useRef<Worker | null>(null)
  const runRef = useRef<RunState | null>(null)
  const fullText = useRef('')

  const updateRun = useCallback((r: RunState | null) => {
    runRef.current = r
    setRun(r)
  }, [])

  const getWorker = useCallback(() => {
    if (workerRef.current) return workerRef.current
    const w = new Worker(new URL('./worker.ts', import.meta.url), {
      type: 'module',
    })
    w.onmessage = (e: MessageEvent) => {
      const m = e.data
      switch (m.ev) {
        case 'started':
          updateRun({
            label: m.label,
            pct: 0,
            resumed: !!m.resumed,
            formatting: false,
          })
          break
        case 'progress':
          updateRun(
            runRef.current ? { ...runRef.current, pct: m.p } : null,
          )
          break
        case 'formatting':
          updateRun(
            runRef.current ? { ...runRef.current, formatting: true } : null,
          )
          break
        case 'done': {
          const label = `${m.label} · ${m.digits.toLocaleString()} digits`
          setBigResult(m.approx, label)
          fullText.current = m.full
          updateRun(null)
          break
        }
        case 'cancelled':
          updateRun(null)
          break
        case 'fulltext':
          void navigator.clipboard?.writeText(m.text).then(() => {
            setCopied(true)
            window.clearTimeout(copiedTimer.current)
            copiedTimer.current = window.setTimeout(() => setCopied(false), 1600)
          })
          break
        case 'error':
          updateRun(null)
          setJobErr('Computation failed')
          window.setTimeout(() => setJobErr(null), 3000)
          break
      }
    }
    workerRef.current = w
    return w
  }, [setBigResult, updateRun])

  // auto-enter long mode when the engine queues a job
  useEffect(() => {
    if (snap.jobSeq > 0 && snap.longJob) {
      fullText.current = ''
      getWorker().postMessage({ cmd: 'run', spec: snap.longJob })
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [snap.jobSeq])

  const cancelRun = useCallback(() => {
    workerRef.current?.postMessage({ cmd: 'cancel' })
  }, [])

  const copyResult = useCallback(() => {
    const local = fullText.current || fullResult()
    if (local) {
      void navigator.clipboard?.writeText(local).then(() => {
        setCopied(true)
        window.clearTimeout(copiedTimer.current)
        copiedTimer.current = window.setTimeout(() => setCopied(false), 1600)
      })
    } else {
      workerRef.current?.postMessage({ cmd: 'fulltext' })
    }
  }, [fullResult])

  // while a job runs only AC/C (cancel) respond; everything else is ignored
  const handlePress = useCallback(
    (id: string) => {
      if (runRef.current) {
        if (id === 'ac' || id === 'c') cancelRun()
        return
      }
      press(id)
    },
    [press, cancelRun],
  )

  useEffect(() => {
    const el = tapeRef.current
    if (el) el.scrollLeft = el.scrollWidth
  }, [snap.tape])

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.metaKey || e.ctrlKey || e.altKey) return
      const id = KEYBOARD_MAP[e.key]
      if (!id) return
      e.preventDefault()
      handlePress(id)
      setFlash(id)
      window.clearTimeout(flashTimer.current)
      flashTimer.current = window.setTimeout(() => setFlash(null), 140)
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [handlePress])

  useEffect(
    () => () => {
      workerRef.current?.terminate()
      window.clearTimeout(flashTimer.current)
      window.clearTimeout(copiedTimer.current)
    },
    [],
  )

  const activeOpId = useMemo(
    () => OP_CHAR_TO_ID[snap.activeOp] ?? '',
    [snap.activeOp],
  )

  const resolve = (def: KeyDef): KeyDef => {
    if (def.id === 'ac' && snap.hasEntry) return { ...def, id: 'c', label: 'C' }
    if (def.id === 'toggle_deg')
      return { ...def, label: snap.deg ? 'Deg' : 'Rad' }
    if (snap.second && def.altId && def.altLabel)
      return { ...def, id: def.altId, label: def.altLabel }
    return def
  }

  const isActive = (def: KeyDef): boolean => {
    if (def.id === '2nd') return snap.second
    if (def.id === 'toggle_deg') return !snap.deg
    return def.id === activeOpId
  }

  const pctText = run
    ? run.pct >= 0.995
      ? '100'
      : run.pct >= 0.99
        ? (run.pct * 100).toFixed(1)
        : String(Math.floor(run.pct * 100))
    : '0'

  return (
    <div className="page" data-theme={theme} data-ready={ready}>
      <main className="calc" aria-label="Scientific calculator">
        <header className="calc-bar">
          <div className="calc-status">
            {!snap.deg && <span className="chip">RAD</span>}
            {snap.hasMemory && <span className="chip">M</span>}
            {snap.error && <span className="chip chip-error">Error</span>}
            {run?.resumed && <span className="chip">Resumed</span>}
            {copied && <span className="chip">Copied</span>}
            {jobErr && <span className="chip chip-error">{jobErr}</span>}
          </div>
          <div className="bar-actions">
            {snap.hasResult && !run && (
              <button
                type="button"
                className="icon-btn"
                onClick={copyResult}
                aria-label="Copy full result"
                title="Copy full result"
              >
                <Copy size={15} />
              </button>
            )}
            {run && (
              <button
                type="button"
                className="icon-btn"
                onClick={cancelRun}
                aria-label="Cancel computation"
                title="Cancel — checkpoint is saved, rerun to resume"
              >
                <X size={15} />
              </button>
            )}
            <button
              type="button"
              className="icon-btn"
              onClick={() => setTheme(theme === 'dark' ? 'light' : 'dark')}
              aria-label="Toggle theme"
            >
              {theme === 'dark' ? <Sun size={15} /> : <Moon size={15} />}
            </button>
          </div>
        </header>

        <div className="display" aria-live="polite">
          {run ? (
            <div className="runview">
              <div className="run-label">
                {run.label}
                {run.resumed && <span className="run-resumed">· resumed</span>}
              </div>
              <div className="run-pct">
                {run.formatting ? '…' : `${pctText}%`}
              </div>
              <div className="run-track">
                <i style={{ transform: `scaleX(${run.pct})` }} />
              </div>
              <div className="run-hint">
                {run.formatting
                  ? 'Formatting result'
                  : 'Computing in background worker · AC to cancel'}
              </div>
            </div>
          ) : (
            <>
              <div className="tape" ref={tapeRef}>
                {snap.tape || '\u00A0'}
              </div>
              <div
                className={`readout${snap.error ? ' is-error' : ''}`}
                style={{
                  fontSize: `min(${displaySize(snap.display.length)}px, ${(displaySize(snap.display.length) / 4).toFixed(1)}cqw)`,
                }}
              >
                {snap.display}
              </div>
            </>
          )}
        </div>

        <div className="pads" aria-disabled={!!run}>
          <div className="pad pad-sci" role="group" aria-label="Scientific functions">
            {SCI_ROWS.flat().map((def) => {
              const r = resolve(def)
              return (
                <CalcKey
                  key={`${def.id}-${r.id}`}
                  def={r}
                  active={isActive(def)}
                  flash={flash === r.id}
                  onPress={handlePress}
                />
              )
            })}
          </div>

          <div className="pad pad-main" role="group" aria-label="Keypad">
            {MAIN_ROWS.flat().map((def) => {
              const r = resolve(def)
              return (
                <CalcKey
                  key={`${def.id}-${r.id}`}
                  def={r}
                  active={isActive(def)}
                  flash={flash === r.id}
                  onPress={handlePress}
                />
              )
            })}
          </div>
        </div>
      </main>

      <p className="footnote">
        Rust engine compiled to WebAssembly{run ? ' · SIMD + OPFS' : ''}
      </p>
    </div>
  )
}
