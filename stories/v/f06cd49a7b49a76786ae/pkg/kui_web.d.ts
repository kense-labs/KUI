/* tslint:disable */
/* eslint-disable */

export function install_input_font(bytes: Uint8Array | number[]): boolean;

export function set_accent(light: number, dark: number): void;

export function set_icon_intent(dark: boolean): void;

export function set_preferences(dark: boolean, reduced: boolean): void;

export function show_story(id: string, chinese: boolean): boolean;

export function start(id: string, dark: boolean, reduced: boolean, chinese: boolean, embedded: boolean): void;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly install_input_font: (a: number, b: number) => number;
    readonly set_accent: (a: number, b: number) => void;
    readonly set_icon_intent: (a: number) => void;
    readonly set_preferences: (a: number, b: number) => void;
    readonly show_story: (a: number, b: number, c: number) => number;
    readonly start: (a: number, b: number, c: number, d: number, e: number, f: number) => void;
    readonly wasm_bindgen_eb94069ace3cac38___convert__closures_____invoke___f32__f32__bool__true_: (a: number, b: number, c: number, d: number) => number;
    readonly wasm_bindgen_eb94069ace3cac38___convert__closures_____invoke___wasm_bindgen_eb94069ace3cac38___JsValue__core_37ee0ce0c4cd57d5___result__Result_____wasm_bindgen_eb94069ace3cac38___JsError___true_: (a: number, b: number, c: any) => [number, number];
    readonly wasm_bindgen_eb94069ace3cac38___convert__closures_____invoke___wasm_bindgen_eb94069ace3cac38___sys__JsNullable_wgpu_57f9fd67ff81b8ca___backend__webgpu__webgpu_sys__gen_GpuError__GpuError___core_37ee0ce0c4cd57d5___result__Result_____wasm_bindgen_eb94069ace3cac38___JsError___true_: (a: number, b: number, c: any) => [number, number];
    readonly wasm_bindgen_eb94069ace3cac38___convert__closures_____invoke___wasm_bindgen_eb94069ace3cac38___sys__JsNullable_wgpu_57f9fd67ff81b8ca___backend__webgpu__webgpu_sys__gen_GpuError__GpuError___core_37ee0ce0c4cd57d5___result__Result_____wasm_bindgen_eb94069ace3cac38___JsError___true__16: (a: number, b: number, c: any) => [number, number];
    readonly wasm_bindgen_eb94069ace3cac38___convert__closures_____invoke___wasm_bindgen_eb94069ace3cac38___sys__JsNullable_wgpu_57f9fd67ff81b8ca___backend__webgpu__webgpu_sys__gen_GpuError__GpuError___core_37ee0ce0c4cd57d5___result__Result_____wasm_bindgen_eb94069ace3cac38___JsError___true__17: (a: number, b: number, c: any) => [number, number];
    readonly wasm_bindgen_eb94069ace3cac38___convert__closures_____invoke___bool______true_: (a: number, b: number, c: number) => void;
    readonly wasm_bindgen_eb94069ace3cac38___convert__closures_____invoke___wasm_bindgen_eb94069ace3cac38___JsValue______true_: (a: number, b: number, c: any) => void;
    readonly wasm_bindgen_eb94069ace3cac38___convert__closures_____invoke___wasm_bindgen_eb94069ace3cac38___JsValue______true__11: (a: number, b: number, c: any) => void;
    readonly wasm_bindgen_eb94069ace3cac38___convert__closures_____invoke___wasm_bindgen_eb94069ace3cac38___JsValue______true__15: (a: number, b: number, c: any) => void;
    readonly wasm_bindgen_eb94069ace3cac38___convert__closures_____invoke___wasm_bindgen_eb94069ace3cac38___JsValue______true__1_: (a: number, b: number, c: any) => void;
    readonly wasm_bindgen_eb94069ace3cac38___convert__closures_____invoke___wgpu_57f9fd67ff81b8ca___backend__webgpu__webgpu_sys__gen_GpuDeviceLostInfo__GpuDeviceLostInfo______true_: (a: number, b: number, c: any) => void;
    readonly wasm_bindgen_eb94069ace3cac38___convert__closures_____invoke___wgpu_57f9fd67ff81b8ca___backend__webgpu__webgpu_sys__gen_GpuDeviceLostInfo__GpuDeviceLostInfo______true__14: (a: number, b: number, c: any) => void;
    readonly wasm_bindgen_eb94069ace3cac38___convert__closures_____invoke_______true_: (a: number, b: number) => void;
    readonly __wbindgen_malloc_command_export: (a: number, b: number) => number;
    readonly __wbindgen_realloc_command_export: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_free_command_export: (a: number, b: number, c: number) => void;
    readonly __externref_table_alloc_command_export: () => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_exn_store_command_export: (a: number) => void;
    readonly __wbindgen_destroy_closure_command_export: (a: number, b: number) => void;
    readonly __externref_table_dealloc_command_export: (a: number) => void;
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
