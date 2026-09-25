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
}

export function useCalculator() {
  const calc = useRef<Calculator | null>(null)
  const [ready, setReady] = useState(false)
  const [snap, setSnap] = useState<CalcSnapshot>(initial)

  const refresh = useCallback(() => {
    const c = calc.current
    if (!c) return
    setSnap({
      display: c.display(),
      tape: c.tape(),
      deg: c.is_deg(),
      second: c.is_second(),
      error: c.is_error(),
      hasEntry: c.has_entry(),
      hasMemory: c.has_memory(),
      activeOp: c.active_op(),
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

  return { ready, snap, press }
}
