/// Long-computation worker.
///
/// Protocol (main -> worker):
///   { cmd: 'run', spec: 'fact:50000' | 'pow:2:1000000' }
///   { cmd: 'cancel' }
///   { cmd: 'fulltext' }            // ask for the full decimal text of last result
///
/// Events (worker -> main):
///   started | progress | formatting | done | cancelled | error | fulltext
///
/// Intermediate state is checkpointed to OPFS (`calc-ckpt/<key>.ckpt`) about
/// twice a second, and on cancel — rerunning the same job silently resumes.
/// Finished results land in `<key>.txt` so the full text survives reload.

import init, { WasmJob } from './wasm/calculator_engine'

const post = (m: Record<string, unknown>) =>
  (self as unknown as Worker).postMessage(m)

let wasmReady: Promise<unknown> | null = null
let cancelled = false
let running = false
let lastFull = ''
let lastKey = ''

// ---- OPFS ----------------------------------------------------------------

type Dir = FileSystemDirectoryHandle
let dirPromise: Promise<Dir | null> | null = null

function getDir(): Promise<Dir | null> {
  dirPromise ??= (async () => {
    try {
      const storage = navigator as Navigator & {
        storage?: { getDirectory?: () => Promise<Dir> }
      }
      const get = storage.storage?.getDirectory
      if (!get) return null
      const root = await get()
      return await root.getDirectoryHandle('calc-ckpt', { create: true })
    } catch {
      return null
    }
  })()
  return dirPromise
}

const fname = (key: string) => key.replace(/[^a-zA-Z0-9_-]/g, '_')

async function readFile(d: Dir, name: string): Promise<Uint8Array | null> {
  try {
    const fh = await d.getFileHandle(name)
    const f = await fh.getFile()
    return new Uint8Array(await f.arrayBuffer())
  } catch {
    return null
  }
}

async function readTextFile(d: Dir, name: string): Promise<string | null> {
  try {
    const fh = await d.getFileHandle(name)
    const f = await fh.getFile()
    return await f.text()
  } catch {
    return null
  }
}

async function writeFile(d: Dir, name: string, data: Uint8Array | string) {
  const fh = await d.getFileHandle(name, { create: true })
  const w = await fh.createWritable()
  await w.write(data as FileSystemWriteChunkType)
  await w.close()
}

async function removeFile(d: Dir, name: string) {
  try {
    await d.removeEntry(name)
  } catch {
    /* absent */
  }
}

/** keep only the newest MAX_CKPT checkpoints so OPFS doesn't accumulate */
async function pruneCheckpoints(d: Dir) {
  const MAX_CKPT = 8
  try {
    const ckpts: { name: string; mtime: number }[] = []
    for await (const [name, handle] of d.entries()) {
      if (!name.endsWith('.ckpt') || handle.kind !== 'file') continue
      try {
        const f = await (handle as FileSystemFileHandle).getFile()
        ckpts.push({ name, mtime: f.lastModified })
      } catch {
        /* skip */
      }
    }
    ckpts.sort((a, b) => b.mtime - a.mtime)
    for (const c of ckpts.slice(MAX_CKPT)) await removeFile(d, c.name)
  } catch {
    /* enumeration unsupported */
  }
}

// ---- job driver ------------------------------------------------------------

const approxOf = (s: string) =>
  s.length <= 12 ? s : `${s[0]}.${s.slice(1, 8)}e${s.length - 1}`

async function run(spec: string) {
  if (running) return
  running = true
  cancelled = false
  try {
    await (wasmReady ??= init())
    const fresh = WasmJob.fromSpec(spec)
    if (!fresh) {
      post({ ev: 'error', message: 'Unknown job' })
      return
    }
    const key = fresh.key()
    lastKey = key
    lastFull = ''

    const d = await getDir()
    let job_ = fresh
    let resumed = false
    if (d) {
      const bytes = await readFile(d, `${fname(key)}.ckpt`)
      if (bytes) {
        const restored = WasmJob.restore(bytes)
        if (restored && restored.key() === key) {
          fresh.free()
          job_ = restored
          resumed = true
        } else restored?.free()
      }
      void pruneCheckpoints(d)
    }

    post({ ev: 'started', key, label: job_.label(), resumed })

    let lastCkpt = performance.now()
    while (!cancelled) {
      const slice = performance.now()
      while (!job_.isDone() && performance.now() - slice < 150) job_.step()
      post({ ev: 'progress', p: job_.progress() })
      if (job_.isDone()) break
      if (d && performance.now() - lastCkpt > 500) {
        try {
          await writeFile(d, `${fname(key)}.ckpt`, job_.checkpoint())
        } catch {
          /* storage quota / transient failure: keep computing */
        }
        lastCkpt = performance.now()
      }
      await new Promise((r) => setTimeout(r, 0)) // let cancel messages arrive
    }

    if (cancelled) {
      if (d) {
        try {
          await writeFile(d, `${fname(key)}.ckpt`, job_.checkpoint())
        } catch {
          /* ignore */
        }
      }
      post({ ev: 'cancelled', key })
      return
    }

    post({ ev: 'formatting', key })
    const dec = job_.toDecimal()
    const label = job_.label()
    job_.free()
    lastFull = dec
    const approx = approxOf(dec)

    if (d) {
      try {
        await writeFile(d, `${fname(key)}.txt`, dec)
        await removeFile(d, `${fname(key)}.ckpt`)
      } catch {
        /* ignore */
      }
    }
    post({ ev: 'done', key, label, approx, digits: dec.length, full: dec })
  } catch (err) {
    post({ ev: 'error', message: String(err) })
  } finally {
    running = false
  }
}

self.onmessage = (e: MessageEvent) => {
  const m = e.data as { cmd: string; spec?: string }
  if (m.cmd === 'run' && m.spec) void run(m.spec)
  else if (m.cmd === 'cancel') cancelled = true
  else if (m.cmd === 'fulltext') {
    if (lastFull) post({ ev: 'fulltext', text: lastFull })
    else {
      void (async () => {
        const d = await getDir()
        const text = d && lastKey ? await readTextFile(d, `${fname(lastKey)}.txt`) : null
        if (text) {
          lastFull = text
          post({ ev: 'fulltext', text })
        } else post({ ev: 'error', message: 'Result unavailable' })
      })()
    }
  }
}

export {}
