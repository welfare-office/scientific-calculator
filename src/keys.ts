export type KeyKind = 'num' | 'fn' | 'op' | 'sci'

export interface KeyDef {
  id: string
  label: string
  kind: KeyKind
  /** alternate action when 2nd is active */
  altId?: string
  altLabel?: string
  wide?: boolean
}

/**
 * Scientific pad: 6 columns x 5 rows, mirrors iOS layout.
 */
export const SCI_ROWS: KeyDef[][] = [
  [
    { id: 'lparen', label: '(', kind: 'sci' },
    { id: 'rparen', label: ')', kind: 'sci' },
    { id: 'mc', label: 'mc', kind: 'sci' },
    { id: 'm+', label: 'm+', kind: 'sci' },
    { id: 'm-', label: 'm-', kind: 'sci' },
    { id: 'mr', label: 'mr', kind: 'sci' },
  ],
  [
    { id: '2nd', label: '2nd', kind: 'sci' },
    { id: 'sq', label: 'x²', kind: 'sci' },
    { id: 'cube', label: 'x³', kind: 'sci' },
    { id: 'pow', label: 'xʸ', kind: 'sci' },
    { id: 'exp', label: 'eˣ', kind: 'sci' },
    { id: 'exp10', label: '10ˣ', kind: 'sci' },
  ],
  [
    { id: 'inv', label: '1/x', kind: 'sci' },
    { id: 'sqrt', label: '√x', kind: 'sci' },
    { id: 'cbrt', label: '∛x', kind: 'sci' },
    { id: 'root', label: 'ʸ√x', kind: 'sci' },
    { id: 'ln', label: 'ln', kind: 'sci', altId: 'log2', altLabel: 'log₂' },
    { id: 'log10', label: 'log', kind: 'sci' },
  ],
  [
    { id: 'fact', label: 'x!', kind: 'sci' },
    { id: 'sin', label: 'sin', kind: 'sci', altId: 'asin', altLabel: 'sin⁻¹' },
    { id: 'cos', label: 'cos', kind: 'sci', altId: 'acos', altLabel: 'cos⁻¹' },
    { id: 'tan', label: 'tan', kind: 'sci', altId: 'atan', altLabel: 'tan⁻¹' },
    { id: 'e', label: 'e', kind: 'sci' },
    { id: 'ee', label: 'EE', kind: 'sci' },
  ],
  [
    { id: 'toggle_deg', label: 'Rad', kind: 'sci' },
    { id: 'sinh', label: 'sinh', kind: 'sci', altId: 'asinh', altLabel: 'sinh⁻¹' },
    { id: 'cosh', label: 'cosh', kind: 'sci', altId: 'acosh', altLabel: 'cosh⁻¹' },
    { id: 'tanh', label: 'tanh', kind: 'sci', altId: 'atanh', altLabel: 'tanh⁻¹' },
    { id: 'pi', label: 'π', kind: 'sci' },
    { id: 'rand', label: 'Rand', kind: 'sci' },
  ],
]

/**
 * Main pad: 4 columns x 5 rows, classic iOS arrangement.
 */
export const MAIN_ROWS: KeyDef[][] = [
  [
    { id: 'ac', label: 'AC', kind: 'fn' },
    { id: 'neg', label: '±', kind: 'fn' },
    { id: 'pct', label: '%', kind: 'fn' },
    { id: 'div', label: '÷', kind: 'op' },
  ],
  [
    { id: '7', label: '7', kind: 'num' },
    { id: '8', label: '8', kind: 'num' },
    { id: '9', label: '9', kind: 'num' },
    { id: 'mul', label: '×', kind: 'op' },
  ],
  [
    { id: '4', label: '4', kind: 'num' },
    { id: '5', label: '5', kind: 'num' },
    { id: '6', label: '6', kind: 'num' },
    { id: 'sub', label: '−', kind: 'op' },
  ],
  [
    { id: '1', label: '1', kind: 'num' },
    { id: '2', label: '2', kind: 'num' },
    { id: '3', label: '3', kind: 'num' },
    { id: 'add', label: '+', kind: 'op' },
  ],
  [
    { id: '0', label: '0', kind: 'num', wide: true },
    { id: 'dot', label: '.', kind: 'num' },
    { id: 'eq', label: '=', kind: 'op' },
  ],
]

/** physical keyboard -> engine key id */
export const KEYBOARD_MAP: Record<string, string> = {
  '0': '0',
  '1': '1',
  '2': '2',
  '3': '3',
  '4': '4',
  '5': '5',
  '6': '6',
  '7': '7',
  '8': '8',
  '9': '9',
  '.': 'dot',
  ',': 'dot',
  '+': 'add',
  '-': 'sub',
  '*': 'mul',
  x: 'mul',
  '/': 'div',
  '^': 'pow',
  '!': 'fact',
  '%': 'pct',
  '(': 'lparen',
  ')': 'rparen',
  Enter: 'eq',
  '=': 'eq',
  Backspace: 'back',
  Escape: 'ac',
  p: 'pi',
  e: 'e',
}
