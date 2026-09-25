import { useEffect, useMemo, useRef, useState } from 'react'
import { Moon, Sun } from '@phosphor-icons/react'
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
  const { ready, snap, press } = useCalculator()
  const [theme, setTheme] = useState<'dark' | 'light'>('dark')
  const [flash, setFlash] = useState<string | null>(null)
  const tapeRef = useRef<HTMLDivElement>(null)
  const flashTimer = useRef<number | undefined>(undefined)

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
      press(id)
      setFlash(id)
      window.clearTimeout(flashTimer.current)
      flashTimer.current = window.setTimeout(() => setFlash(null), 140)
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [press])

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

  return (
    <div className="page" data-theme={theme} data-ready={ready}>
      <main className="calc" aria-label="Scientific calculator">
        <header className="calc-bar">
          <div className="calc-status">
            {!snap.deg && <span className="chip">RAD</span>}
            {snap.hasMemory && <span className="chip">M</span>}
            {snap.error && <span className="chip chip-error">Error</span>}
          </div>
          <button
            type="button"
            className="icon-btn"
            onClick={() => setTheme(theme === 'dark' ? 'light' : 'dark')}
            aria-label="Toggle theme"
          >
            {theme === 'dark' ? <Sun size={15} /> : <Moon size={15} />}
          </button>
        </header>

        <div className="display" aria-live="polite">
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
        </div>

        <div className="pads">
          <div className="pad pad-sci" role="group" aria-label="Scientific functions">
            {SCI_ROWS.flat().map((def) => {
              const r = resolve(def)
              return (
                <CalcKey
                  key={`${def.id}-${r.id}`}
                  def={r}
                  active={isActive(def)}
                  flash={flash === r.id}
                  onPress={press}
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
                  onPress={press}
                />
              )
            })}
          </div>
        </div>
      </main>

      <p className="footnote">Rust engine compiled to WebAssembly</p>
    </div>
  )
}
