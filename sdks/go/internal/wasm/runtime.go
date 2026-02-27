// Package wasm provides a wazero-based runtime for the Tenor WASM bridge.
// It loads the embedded tenor_eval.wasm binary and exposes methods for
// calling its exported C-ABI functions.
//
// The WASM binary is produced by crates/tenor-wasm-bridge (wasm32-unknown-unknown).
// It uses an alloc/dealloc/get_result_ptr/get_result_len memory protocol for
// passing strings without wasm-bindgen or JS glue code.
package wasm

import (
	"context"
	_ "embed"
	"fmt"
	"sync"

	"github.com/tetratelabs/wazero"
	"github.com/tetratelabs/wazero/api"
)

// tenor_eval.wasm is the compiled WASM bridge binary.
// It must be built via sdks/go/scripts/build-wasm.sh before building this package.
//
//go:embed tenor_eval.wasm
var wasmBinary []byte

// Runtime manages the wazero WASM runtime and the loaded Tenor module instance.
// It is safe for concurrent use; all WASM calls are serialised by a mutex
// because the WASM module is single-threaded.
type Runtime struct {
	mu      sync.Mutex
	runtime wazero.Runtime
	module  api.Module
	ctx     context.Context
}

// NewRuntime creates a new wazero runtime and instantiates the Tenor WASM module.
// The caller must call Close() when done.
func NewRuntime(ctx context.Context) (*Runtime, error) {
	r := wazero.NewRuntime(ctx)

	// Instantiate with an empty module name so it does not conflict with
	// other instances in the same runtime if one is ever shared.
	mod, err := r.InstantiateWithConfig(ctx, wasmBinary,
		wazero.NewModuleConfig().WithName("tenor-eval"))
	if err != nil {
		_ = r.Close(ctx)
		return nil, fmt.Errorf("failed to instantiate Tenor WASM module: %w", err)
	}

	return &Runtime{
		runtime: r,
		module:  mod,
		ctx:     ctx,
	}, nil
}

// CallOneArg calls a WASM function that takes a single string argument (ptr, len)
// and writes its result to the result buffer.
// Returns the JSON result string from get_result_ptr/get_result_len.
func (rt *Runtime) CallOneArg(funcName string, arg string) (string, error) {
	rt.mu.Lock()
	defer rt.mu.Unlock()

	ptr, free, err := rt.writeString(arg)
	if err != nil {
		return "", err
	}
	defer free()

	fn := rt.module.ExportedFunction(funcName)
	if fn == nil {
		return "", fmt.Errorf("WASM function %q not found", funcName)
	}

	if _, err := fn.Call(rt.ctx, uint64(ptr), uint64(len(arg))); err != nil {
		return "", fmt.Errorf("WASM call %q failed: %w", funcName, err)
	}

	return rt.readResult()
}

// CallHandleOneArg calls a WASM function with (handle u32, arg_ptr, arg_len).
func (rt *Runtime) CallHandleOneArg(funcName string, handle uint32, arg string) (string, error) {
	rt.mu.Lock()
	defer rt.mu.Unlock()

	ptr, free, err := rt.writeString(arg)
	if err != nil {
		return "", err
	}
	defer free()

	fn := rt.module.ExportedFunction(funcName)
	if fn == nil {
		return "", fmt.Errorf("WASM function %q not found", funcName)
	}

	if _, err := fn.Call(rt.ctx, uint64(handle), uint64(ptr), uint64(len(arg))); err != nil {
		return "", fmt.Errorf("WASM call %q failed: %w", funcName, err)
	}

	return rt.readResult()
}

// CallHandleThreeArgs calls a WASM function with
// (handle u32, arg1_ptr, arg1_len, arg2_ptr, arg2_len, arg3_ptr, arg3_len).
// This is used for compute_action_space (facts, entity_states, persona).
func (rt *Runtime) CallHandleThreeArgs(
	funcName string,
	handle uint32,
	arg1, arg2, arg3 string,
) (string, error) {
	rt.mu.Lock()
	defer rt.mu.Unlock()

	ptr1, free1, err := rt.writeString(arg1)
	if err != nil {
		return "", err
	}
	defer free1()

	ptr2, free2, err := rt.writeString(arg2)
	if err != nil {
		return "", err
	}
	defer free2()

	ptr3, free3, err := rt.writeString(arg3)
	if err != nil {
		return "", err
	}
	defer free3()

	fn := rt.module.ExportedFunction(funcName)
	if fn == nil {
		return "", fmt.Errorf("WASM function %q not found", funcName)
	}

	params := []uint64{
		uint64(handle),
		uint64(ptr1), uint64(len(arg1)),
		uint64(ptr2), uint64(len(arg2)),
		uint64(ptr3), uint64(len(arg3)),
	}
	if _, err := fn.Call(rt.ctx, params...); err != nil {
		return "", fmt.Errorf("WASM call %q failed: %w", funcName, err)
	}

	return rt.readResult()
}

// CallHandleFiveArgs calls a WASM function with
// (handle, a1_ptr, a1_len, a2_ptr, a2_len, a3_ptr, a3_len, a4_ptr, a4_len, a5_ptr, a5_len).
// This is used for simulate_flow_with_bindings.
func (rt *Runtime) CallHandleFiveArgs(
	funcName string,
	handle uint32,
	arg1, arg2, arg3, arg4, arg5 string,
) (string, error) {
	rt.mu.Lock()
	defer rt.mu.Unlock()

	args := []string{arg1, arg2, arg3, arg4, arg5}
	ptrs := make([]uint32, len(args))
	frees := make([]func(), len(args))

	for i, arg := range args {
		ptr, free, err := rt.writeStringUnlocked(arg)
		if err != nil {
			// Free already-allocated buffers
			for j := 0; j < i; j++ {
				frees[j]()
			}
			return "", err
		}
		ptrs[i] = ptr
		frees[i] = free
	}
	defer func() {
		for _, free := range frees {
			free()
		}
	}()

	fn := rt.module.ExportedFunction(funcName)
	if fn == nil {
		return "", fmt.Errorf("WASM function %q not found", funcName)
	}

	params := []uint64{uint64(handle)}
	for i, ptr := range ptrs {
		params = append(params, uint64(ptr), uint64(len(args[i])))
	}

	if _, err := fn.Call(rt.ctx, params...); err != nil {
		return "", fmt.Errorf("WASM call %q failed: %w", funcName, err)
	}

	return rt.readResult()
}

// CallHandleFourArgs calls a WASM function with
// (handle, a1_ptr, a1_len, a2_ptr, a2_len, a3_ptr, a3_len, a4_ptr, a4_len).
// This is used for simulate_flow (no instance_bindings).
func (rt *Runtime) CallHandleFourArgs(
	funcName string,
	handle uint32,
	arg1, arg2, arg3, arg4 string,
) (string, error) {
	rt.mu.Lock()
	defer rt.mu.Unlock()

	args := []string{arg1, arg2, arg3, arg4}
	ptrs := make([]uint32, len(args))
	frees := make([]func(), len(args))

	for i, arg := range args {
		ptr, free, err := rt.writeStringUnlocked(arg)
		if err != nil {
			for j := 0; j < i; j++ {
				frees[j]()
			}
			return "", err
		}
		ptrs[i] = ptr
		frees[i] = free
	}
	defer func() {
		for _, free := range frees {
			free()
		}
	}()

	fn := rt.module.ExportedFunction(funcName)
	if fn == nil {
		return "", fmt.Errorf("WASM function %q not found", funcName)
	}

	params := []uint64{uint64(handle)}
	for i, ptr := range ptrs {
		params = append(params, uint64(ptr), uint64(len(args[i])))
	}

	if _, err := fn.Call(rt.ctx, params...); err != nil {
		return "", fmt.Errorf("WASM call %q failed: %w", funcName, err)
	}

	return rt.readResult()
}

// Close releases all WASM runtime resources.
func (rt *Runtime) Close() error {
	return rt.runtime.Close(rt.ctx)
}

// writeString allocates memory in the WASM module for arg, writes the bytes,
// and returns a pointer, a cleanup function, and any error.
// Acquires the mutex — do not call from within a locked region.
func (rt *Runtime) writeString(arg string) (uint32, func(), error) {
	return rt.writeStringUnlocked(arg)
}

// writeStringUnlocked is the unlocked version — call only when rt.mu is held.
func (rt *Runtime) writeStringUnlocked(arg string) (uint32, func(), error) {
	if len(arg) == 0 {
		// Return a valid pointer of length 0. The WASM alloc(0) behaviour is
		// unspecified; use offset 0 (safe because len is 0, so the pointer
		// is never dereferenced).
		return 0, func() {}, nil
	}

	allocFn := rt.module.ExportedFunction("alloc")
	deallocFn := rt.module.ExportedFunction("dealloc")
	if allocFn == nil {
		return 0, nil, fmt.Errorf("WASM function \"alloc\" not found")
	}

	results, err := allocFn.Call(rt.ctx, uint64(len(arg)))
	if err != nil {
		return 0, nil, fmt.Errorf("WASM alloc(%d) failed: %w", len(arg), err)
	}
	ptr := uint32(results[0])

	mem := rt.module.Memory()
	if ok := mem.Write(ptr, []byte(arg)); !ok {
		return 0, nil, fmt.Errorf("failed to write %d bytes to WASM memory at offset %d", len(arg), ptr)
	}

	free := func() {
		if deallocFn != nil {
			_, _ = deallocFn.Call(rt.ctx, uint64(ptr), uint64(len(arg)))
		}
	}
	return ptr, free, nil
}

// readResult reads the result from the WASM result buffer.
// Must be called while holding rt.mu.
func (rt *Runtime) readResult() (string, error) {
	getPtrFn := rt.module.ExportedFunction("get_result_ptr")
	getLenFn := rt.module.ExportedFunction("get_result_len")

	if getPtrFn == nil || getLenFn == nil {
		return "", fmt.Errorf("WASM result functions not found")
	}

	ptrResult, err := getPtrFn.Call(rt.ctx)
	if err != nil {
		return "", fmt.Errorf("get_result_ptr failed: %w", err)
	}
	lenResult, err := getLenFn.Call(rt.ctx)
	if err != nil {
		return "", fmt.Errorf("get_result_len failed: %w", err)
	}

	resultPtr := uint32(ptrResult[0])
	resultLen := uint32(lenResult[0])

	if resultLen == 0 {
		return "", nil
	}

	mem := rt.module.Memory()
	bytes, ok := mem.Read(resultPtr, resultLen)
	if !ok {
		return "", fmt.Errorf("failed to read %d bytes from WASM memory at offset %d", resultLen, resultPtr)
	}

	// Copy because the WASM memory may be overwritten by the next call.
	result := make([]byte, resultLen)
	copy(result, bytes)
	return string(result), nil
}
