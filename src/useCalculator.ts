import { useCallback, useEffect, useRef, useState } from 'react'
import init, { Calculator } from './wasm/calculator_engine'

export interface CalcSnapshot {
  display: string
  tape: string
  deg: boolean
  second: boolean
  error: boolean
  hasEntry: boolean
  hasMemory: boolean
  activeOp: string
  /** "" when no job queued; drained once — watch jobSeq to react */
  longJob: string
  /** increments each time a long job is queued */
  jobSeq: number
  /** a big (non-f64) result is being displayed; fullResult() may have text */
  hasResult: boolean
}

const initial: CalcSnapshot = {
  display: '0',
  tape: '',
  deg: true,
  second: false,
  error: false,
  hasEntry: false,
  hasMemory: false,
  activeOp: '',
  longJob: '',
  jobSeq: 0,
  hasResult: false,
}

export function useCalculator() {
  const calc = useRef<Calculator | null>(null)
  const jobSeq = useRef(0)
  const [ready, setReady] = useState(false)
  const [snap, setSnap] = useState<CalcSnapshot>(initial)

  const refresh = useCallback(() => {
    const c = calc.current
    if (!c) return
    const spec = c.take_long_job()
    if (spec) jobSeq.current += 1
    setSnap({
      display: c.display(),
      tape: c.tape(),
      deg: c.is_deg(),
      second: c.is_second(),
      error: c.is_error(),
      hasEntry: c.has_entry(),
      hasMemory: c.has_memory(),
      activeOp: c.active_op(),
      longJob: spec,
      jobSeq: jobSeq.current,
      hasResult: c.has_big_result(),
    })
  }, [])

  useEffect(() => {
    let alive = true
    init().then(() => {
      if (!alive) return
      calc.current = new Calculator()
      setReady(true)
      refresh()
    })
    return () => {
      alive = false
    }
  }, [refresh])

  const press = useCallback(
    (key: string) => {
      calc.current?.press(key)
      refresh()
    },
    [refresh],
  )

  const setBigResult = useCallback(
    (approx: string, label: string) => {
      calc.current?.set_big_result(approx, label)
      refresh()
    },
    [refresh],
  )

  const fullResult = useCallback(() => calc.current?.full_result() ?? '', [])

  return { ready, snap, press, setBigResult, fullResult }
}
