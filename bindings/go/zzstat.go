package zzstat

import (
	"fmt"
	"runtime"
	"github.com/ebitengine/purego"
)

var (
	lib uintptr

	zzstat_context_new                            func() uintptr
	zzstat_context_free                           func(uintptr)
	zzstat_context_set_float                      func(uintptr, *byte, float64) int32
	zzstat_context_get_float                      func(uintptr, *byte, float64) float64
	zzstat_context_set_bool                       func(uintptr, *byte, bool) int32
	zzstat_context_get_bool                       func(uintptr, *byte, bool) bool

	zzstat_resolver_new                           func() uintptr
	zzstat_resolver_free                          func(uintptr)
	zzstat_resolver_register_constant_source      func(uintptr, *byte, float64) int32
	zzstat_resolver_register_map_source           func(uintptr, **byte, *float64, int) int32
	zzstat_resolver_register_additive_transform   func(uintptr, *byte, byte, byte, float64) int32
	zzstat_resolver_register_multiplicative_transform func(uintptr, *byte, byte, byte, float64) int32
	zzstat_resolver_register_clamp_transform      func(uintptr, *byte, byte, byte, bool, float64, bool, float64) int32
	zzstat_resolver_register_scaling_transform    func(uintptr, *byte, byte, byte, *byte, float64) int32
	zzstat_resolver_register_conditional_multiplicative_transform func(uintptr, *byte, byte, byte, uintptr, uintptr, uintptr, float64, *byte) int32
	zzstat_resolver_register_conditional_additive_transform func(uintptr, *byte, byte, byte, uintptr, uintptr, uintptr, float64, *byte) int32
	zzstat_resolver_invalidate                    func(uintptr, *byte) int32
	zzstat_resolver_invalidate_all                func(uintptr) int32
	zzstat_resolver_resolve                       func(uintptr, *byte, uintptr, *float64) int32

	zzstat_combat_evaluate                        func(*byte, uintptr, uintptr, uintptr, uintptr, uintptr, uintptr, *float64) int32
)

// LoadLibrary allows manually specifying the library path.
func LoadLibrary(libPath string) error {
	var err error
	lib, err = purego.Dlopen(libPath, purego.RTLD_NOW|purego.RTLD_GLOBAL)
	if err != nil {
		return err
	}
	bindFunctions()
	return nil
}

func bindFunctions() {
	purego.RegisterLibFunc(&zzstat_context_new, lib, "zzstat_context_new")
	purego.RegisterLibFunc(&zzstat_context_free, lib, "zzstat_context_free")
	purego.RegisterLibFunc(&zzstat_context_set_float, lib, "zzstat_context_set_float")
	purego.RegisterLibFunc(&zzstat_context_get_float, lib, "zzstat_context_get_float")
	purego.RegisterLibFunc(&zzstat_context_set_bool, lib, "zzstat_context_set_bool")
	purego.RegisterLibFunc(&zzstat_context_get_bool, lib, "zzstat_context_get_bool")

	purego.RegisterLibFunc(&zzstat_resolver_new, lib, "zzstat_resolver_new")
	purego.RegisterLibFunc(&zzstat_resolver_free, lib, "zzstat_resolver_free")
	purego.RegisterLibFunc(&zzstat_resolver_register_constant_source, lib, "zzstat_resolver_register_constant_source")
	purego.RegisterLibFunc(&zzstat_resolver_register_map_source, lib, "zzstat_resolver_register_map_source")
	purego.RegisterLibFunc(&zzstat_resolver_register_additive_transform, lib, "zzstat_resolver_register_additive_transform")
	purego.RegisterLibFunc(&zzstat_resolver_register_multiplicative_transform, lib, "zzstat_resolver_register_multiplicative_transform")
	purego.RegisterLibFunc(&zzstat_resolver_register_clamp_transform, lib, "zzstat_resolver_register_clamp_transform")
	purego.RegisterLibFunc(&zzstat_resolver_register_scaling_transform, lib, "zzstat_resolver_register_scaling_transform")
	purego.RegisterLibFunc(&zzstat_resolver_register_conditional_multiplicative_transform, lib, "zzstat_resolver_register_conditional_multiplicative_transform")
	purego.RegisterLibFunc(&zzstat_resolver_register_conditional_additive_transform, lib, "zzstat_resolver_register_conditional_additive_transform")
	purego.RegisterLibFunc(&zzstat_resolver_invalidate, lib, "zzstat_resolver_invalidate")
	purego.RegisterLibFunc(&zzstat_resolver_invalidate_all, lib, "zzstat_resolver_invalidate_all")
	purego.RegisterLibFunc(&zzstat_resolver_resolve, lib, "zzstat_resolver_resolve")

	purego.RegisterLibFunc(&zzstat_combat_evaluate, lib, "zzstat_combat_evaluate")
}

func init() {
	// Try loading from standard paths
	var libPath string
	switch runtime.GOOS {
	case "darwin":
		libPath = "libzzstat_ffi.dylib"
	case "windows":
		libPath = "zzstat_ffi.dll"
	default:
		libPath = "libzzstat_ffi.so"
	}

	// Try standard library load
	var err error
	lib, err = purego.Dlopen(libPath, purego.RTLD_NOW|purego.RTLD_GLOBAL)
	if err != nil {
		// Try relative paths for development and testing
		lib, err = purego.Dlopen("./target/release/libzzstat_ffi.so", purego.RTLD_NOW|purego.RTLD_GLOBAL)
		if err != nil {
			lib, err = purego.Dlopen("../../target/release/libzzstat_ffi.so", purego.RTLD_NOW|purego.RTLD_GLOBAL)
			if err != nil {
				// Don't panic immediately, allow user to call LoadLibrary manually if needed.
				return
			}
		}
	}
	bindFunctions()
}

// Context wrapper
type Context struct {
	ptr uintptr
}

func NewContext() *Context {
	return &Context{ptr: zzstat_context_new()}
}

func (c *Context) Free() {
	if c.ptr != 0 {
		zzstat_context_free(c.ptr)
		c.ptr = 0
	}
}

func (c *Context) SetFloat(key string, val float64) {
	kBytes := []byte(key + "\x00")
	zzstat_context_set_float(c.ptr, &kBytes[0], val)
}

func (c *Context) GetFloat(key string, defaultVal float64) float64 {
	kBytes := []byte(key + "\x00")
	return zzstat_context_get_float(c.ptr, &kBytes[0], defaultVal)
}

func (c *Context) SetBool(key string, val bool) {
	kBytes := []byte(key + "\x00")
	zzstat_context_set_bool(c.ptr, &kBytes[0], val)
}

func (c *Context) GetBool(key string, defaultVal bool) bool {
	kBytes := []byte(key + "\x00")
	return zzstat_context_get_bool(c.ptr, &kBytes[0], defaultVal)
}

// Resolver wrapper
type Resolver struct {
	ptr uintptr
}

func NewResolver() *Resolver {
	return &Resolver{ptr: zzstat_resolver_new()}
}

func (r *Resolver) Free() {
	if r.ptr != 0 {
		zzstat_resolver_free(r.ptr)
		r.ptr = 0
	}
}

func (r *Resolver) RegisterConstantSource(statID string, value float64) {
	idBytes := []byte(statID + "\x00")
	zzstat_resolver_register_constant_source(r.ptr, &idBytes[0], value)
}

func (r *Resolver) RegisterMapSource(source map[string]float64) {
	lenMap := len(source)
	if lenMap == 0 {
		return
	}
	keys := make([]*byte, 0, lenMap)
	values := make([]float64, 0, lenMap)
	// Keep backing byte arrays alive during function call
	backingBytes := make([][]byte, 0, lenMap)
	for k, v := range source {
		b := []byte(k + "\x00")
		backingBytes = append(backingBytes, b)
		keys = append(keys, &b[0])
		values = append(values, v)
	}
	zzstat_resolver_register_map_source(r.ptr, &keys[0], &values[0], lenMap)
}

func (r *Resolver) Invalidate(statID string) {
	idBytes := []byte(statID + "\x00")
	zzstat_resolver_invalidate(r.ptr, &idBytes[0])
}

func (r *Resolver) InvalidateAll() {
	zzstat_resolver_invalidate_all(r.ptr)
}

// Phases and rules mapping
const (
	PhaseAdditive       = 0
	PhaseMultiplicative = 1
	PhaseFinal          = 2
)

const (
	RuleOverride       = 0
	RuleAdditive       = 1
	RuleMultiplicative = 2
	RuleMin            = 3
	RuleMax            = 4
	RuleMinMax         = 5
)

func (r *Resolver) RegisterAdditiveTransform(statID string, phase byte, rule byte, value float64) {
	idBytes := []byte(statID + "\x00")
	zzstat_resolver_register_additive_transform(r.ptr, &idBytes[0], phase, rule, value)
}

func (r *Resolver) RegisterMultiplicativeTransform(statID string, phase byte, rule byte, value float64) {
	idBytes := []byte(statID + "\x00")
	zzstat_resolver_register_multiplicative_transform(r.ptr, &idBytes[0], phase, rule, value)
}

func (r *Resolver) RegisterClampTransform(statID string, phase byte, rule byte, hasMin bool, minVal float64, hasMax bool, maxVal float64) {
	idBytes := []byte(statID + "\x00")
	zzstat_resolver_register_clamp_transform(r.ptr, &idBytes[0], phase, rule, hasMin, minVal, hasMax, maxVal)
}

func (r *Resolver) RegisterScalingTransform(statID string, phase byte, rule byte, dependency string, scaleFactor float64) {
	idBytes := []byte(statID + "\x00")
	depBytes := []byte(dependency + "\x00")
	zzstat_resolver_register_scaling_transform(r.ptr, &idBytes[0], phase, rule, &depBytes[0], scaleFactor)
}

func (r *Resolver) RegisterConditionalMultiplicativeTransform(
	statID string,
	phase byte,
	rule byte,
	condition func(ctx *Context) bool,
	multiplier float64,
	description string,
) {
	idBytes := []byte(statID + "\x00")
	descBytes := []byte(description + "\x00")

	cb := purego.NewCallback(func(ctxPtr uintptr, userData uintptr) bool {
		ctx := &Context{ptr: ctxPtr}
		return condition(ctx)
	})

	zzstat_resolver_register_conditional_multiplicative_transform(
		r.ptr,
		&idBytes[0],
		phase,
		rule,
		cb,
		0,
		0,
		multiplier,
		&descBytes[0],
	)
}

func (r *Resolver) RegisterConditionalAdditiveTransform(
	statID string,
	phase byte,
	rule byte,
	condition func(ctx *Context) bool,
	bonus float64,
	description string,
) {
	idBytes := []byte(statID + "\x00")
	descBytes := []byte(description + "\x00")

	cb := purego.NewCallback(func(ctxPtr uintptr, userData uintptr) bool {
		ctx := &Context{ptr: ctxPtr}
		return condition(ctx)
	})

	zzstat_resolver_register_conditional_additive_transform(
		r.ptr,
		&idBytes[0],
		phase,
		rule,
		cb,
		0,
		0,
		bonus,
		&descBytes[0],
	)
}

func (r *Resolver) Resolve(statID string, ctx *Context) (float64, error) {
	idBytes := []byte(statID + "\x00")
	var outVal float64
	res := zzstat_resolver_resolve(r.ptr, &idBytes[0], ctx.ptr, &outVal)
	if res != 0 {
		return 0, fmt.Errorf("stat resolution failed with error code: %d", res)
	}
	return outVal, nil
}

func EvaluateCombat(formulaJSON string, attacker *Resolver, attackerCtx *Context, defender *Resolver, defenderCtx *Context, rng func() float64) (float64, error) {
	jsonBytes := []byte(formulaJSON + "\x00")
	var outVal float64

	var cb uintptr
	if rng != nil {
		cb = purego.NewCallback(func(userData uintptr) float64 {
			return rng()
		})
	}

	res := zzstat_combat_evaluate(&jsonBytes[0], attacker.ptr, attackerCtx.ptr, defender.ptr, defenderCtx.ptr, cb, 0, &outVal)
	if res != 0 {
		return 0, fmt.Errorf("combat evaluation failed with error code: %d", res)
	}
	return outVal, nil
}
