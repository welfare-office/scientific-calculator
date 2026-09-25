/* tslint:disable */
/* eslint-disable */

export class Calculator {
    free(): void;
    [Symbol.dispose](): void;
    /**
     * name of the pending binary operator for highlight, "" if none
     */
    active_op(): string;
    display(): string;
    /**
     * full decimal text when computed synchronously (async path streams it
     * through the worker instead); "" when unavailable
     */
    full_result(): string;
    has_big_result(): boolean;
    has_entry(): boolean;
    has_memory(): boolean;
    is_deg(): boolean;
    is_error(): boolean;
    is_second(): boolean;
    constructor();
    /**
     * single entry point: press(id) where id is a key identifier
     */
    press(key: string): void;
    /**
     * called by the UI when a worker finishes: show approx + tape label
     */
    set_big_result(approx: string, label: string): void;
    /**
     * drains a queued long-computation request; "" when none
     */
    take_long_job(): string;
    /**
     * expression preview shown above the main display
     */
    tape(): string;
}

/**
 * Async computation job exposed to JS: the worker drives `step()` in a loop,
 * persists `checkpoint()` bytes to OPFS, and can `restore()` to resume.
 */
export class WasmJob {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    checkpoint(): Uint8Array;
    /**
     * Parse a job spec "fact:100000" / "pow:2:999999"; undefined if unknown.
     */
    static fromSpec(spec: string): WasmJob | undefined;
    isDone(): boolean;
    key(): string;
    label(): string;
    static newFactorial(n: number): WasmJob;
    static newPow(base: number, exp: number): WasmJob;
    progress(): number;
    /**
     * Restore from checkpoint bytes; returns undefined if the bytes are invalid.
     */
    static restore(bytes: Uint8Array): WasmJob | undefined;
    /**
     * One chunk of work. Returns progress in [0,1]; 1.0 means finished.
     */
    step(): number;
    /**
     * Full decimal expansion of the result. Expensive - call once when done.
     */
    toDecimal(): string;
}

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_calculator_free: (a: number, b: number) => void;
    readonly __wbg_wasmjob_free: (a: number, b: number) => void;
    readonly calculator_active_op: (a: number) => [number, number];
    readonly calculator_display: (a: number) => [number, number];
    readonly calculator_full_result: (a: number) => [number, number];
    readonly calculator_has_big_result: (a: number) => number;
    readonly calculator_has_entry: (a: number) => number;
    readonly calculator_has_memory: (a: number) => number;
    readonly calculator_is_deg: (a: number) => number;
    readonly calculator_is_error: (a: number) => number;
    readonly calculator_is_second: (a: number) => number;
    readonly calculator_new: () => number;
    readonly calculator_press: (a: number, b: number, c: number) => void;
    readonly calculator_set_big_result: (a: number, b: number, c: number, d: number, e: number) => void;
    readonly calculator_take_long_job: (a: number) => [number, number];
    readonly calculator_tape: (a: number) => [number, number];
    readonly wasmjob_checkpoint: (a: number) => [number, number];
    readonly wasmjob_fromSpec: (a: number, b: number) => number;
    readonly wasmjob_isDone: (a: number) => number;
    readonly wasmjob_key: (a: number) => [number, number];
    readonly wasmjob_label: (a: number) => [number, number];
    readonly wasmjob_newFactorial: (a: number) => number;
    readonly wasmjob_newPow: (a: number, b: number) => number;
    readonly wasmjob_progress: (a: number) => number;
    readonly wasmjob_restore: (a: number, b: number) => number;
    readonly wasmjob_step: (a: number) => number;
    readonly wasmjob_toDecimal: (a: number) => [number, number];
    readonly __wbindgen_exn_store: (a: number) => void;
    readonly __externref_table_alloc: () => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
