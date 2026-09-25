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
     * expression preview shown above the main display
     */
    tape(): string;
}

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_calculator_free: (a: number, b: number) => void;
    readonly calculator_active_op: (a: number) => [number, number];
    readonly calculator_display: (a: number) => [number, number];
    readonly calculator_has_entry: (a: number) => number;
    readonly calculator_has_memory: (a: number) => number;
    readonly calculator_is_deg: (a: number) => number;
    readonly calculator_is_error: (a: number) => number;
    readonly calculator_is_second: (a: number) => number;
    readonly calculator_new: () => number;
    readonly calculator_press: (a: number, b: number, c: number) => void;
    readonly calculator_tape: (a: number) => [number, number];
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
