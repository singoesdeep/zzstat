import os
import sys
import ctypes

# Try to load the shared library
lib_path = None
# Check relative paths
search_paths = [
    os.path.join(os.path.dirname(__file__), "libzzstat_ffi.so"),
    os.path.join(os.path.dirname(__file__), "../../../target/release/libzzstat_ffi.so"),
    os.path.join(os.path.dirname(__file__), "../../../target/release/libzzstat_ffi.dylib"),
    os.path.join(os.path.dirname(__file__), "../../../target/release/zzstat_ffi.dll"),
    "libzzstat_ffi.so",
    "libzzstat_ffi.dylib",
    "zzstat_ffi.dll",
]

for path in search_paths:
    if os.path.exists(path):
        lib_path = path
        break

if not lib_path:
    # Try system load
    try:
        if sys.platform == "win32":
            lib = ctypes.CDLL("zzstat_ffi.dll")
        elif sys.platform == "darwin":
            lib = ctypes.CDLL("libzzstat_ffi.dylib")
        else:
            lib = ctypes.CDLL("libzzstat_ffi.so")
    except OSError:
        lib = None
else:
    lib = ctypes.CDLL(lib_path)

if lib:
    # Context API
    lib.zzstat_context_new.restype = ctypes.c_void_p
    lib.zzstat_context_new.argtypes = []

    lib.zzstat_context_free.restype = None
    lib.zzstat_context_free.argtypes = [ctypes.c_void_p]

    lib.zzstat_context_set_float.restype = ctypes.c_int32
    lib.zzstat_context_set_float.argtypes = [ctypes.c_void_p, ctypes.c_char_p, ctypes.c_double]

    lib.zzstat_context_get_float.restype = ctypes.c_double
    lib.zzstat_context_get_float.argtypes = [ctypes.c_void_p, ctypes.c_char_p, ctypes.c_double]

    lib.zzstat_context_set_bool.restype = ctypes.c_int32
    lib.zzstat_context_set_bool.argtypes = [ctypes.c_void_p, ctypes.c_char_p, ctypes.c_bool]

    lib.zzstat_context_get_bool.restype = ctypes.c_bool
    lib.zzstat_context_get_bool.argtypes = [ctypes.c_void_p, ctypes.c_char_p, ctypes.c_bool]

    # Resolver API
    lib.zzstat_resolver_new.restype = ctypes.c_void_p
    lib.zzstat_resolver_new.argtypes = []

    lib.zzstat_resolver_free.restype = None
    lib.zzstat_resolver_free.argtypes = [ctypes.c_void_p]

    lib.zzstat_resolver_register_constant_source.restype = ctypes.c_int32
    lib.zzstat_resolver_register_constant_source.argtypes = [ctypes.c_void_p, ctypes.c_char_p, ctypes.c_double]

    lib.zzstat_resolver_register_map_source.restype = ctypes.c_int32
    lib.zzstat_resolver_register_map_source.argtypes = [
        ctypes.c_void_p,
        ctypes.POINTER(ctypes.c_char_p),
        ctypes.POINTER(ctypes.c_double),
        ctypes.c_size_t
    ]

    lib.zzstat_resolver_invalidate.restype = ctypes.c_int32
    lib.zzstat_resolver_invalidate.argtypes = [ctypes.c_void_p, ctypes.c_char_p]

    lib.zzstat_resolver_invalidate_all.restype = ctypes.c_int32
    lib.zzstat_resolver_invalidate_all.argtypes = [ctypes.c_void_p]

    lib.zzstat_resolver_register_additive_transform.restype = ctypes.c_int32
    lib.zzstat_resolver_register_additive_transform.argtypes = [
        ctypes.c_void_p, ctypes.c_char_p, ctypes.c_ubyte, ctypes.c_ubyte, ctypes.c_double
    ]

    lib.zzstat_resolver_register_multiplicative_transform.restype = ctypes.c_int32
    lib.zzstat_resolver_register_multiplicative_transform.argtypes = [
        ctypes.c_void_p, ctypes.c_char_p, ctypes.c_ubyte, ctypes.c_ubyte, ctypes.c_double
    ]

    lib.zzstat_resolver_register_clamp_transform.restype = ctypes.c_int32
    lib.zzstat_resolver_register_clamp_transform.argtypes = [
        ctypes.c_void_p, ctypes.c_char_p, ctypes.c_ubyte, ctypes.c_ubyte, ctypes.c_bool, ctypes.c_double, ctypes.c_bool, ctypes.c_double
    ]

    lib.zzstat_resolver_register_scaling_transform.restype = ctypes.c_int32
    lib.zzstat_resolver_register_scaling_transform.argtypes = [
        ctypes.c_void_p, ctypes.c_char_p, ctypes.c_ubyte, ctypes.c_ubyte, ctypes.c_char_p, ctypes.c_double
    ]

    # Callbacks
    CONDITION_CALLBACK = ctypes.CFUNCTYPE(ctypes.c_bool, ctypes.c_void_p, ctypes.c_void_p)
    FREE_USER_DATA_CALLBACK = ctypes.CFUNCTYPE(None, ctypes.c_void_p)

    lib.zzstat_resolver_register_conditional_multiplicative_transform.restype = ctypes.c_int32
    lib.zzstat_resolver_register_conditional_multiplicative_transform.argtypes = [
        ctypes.c_void_p, ctypes.c_char_p, ctypes.c_ubyte, ctypes.c_ubyte,
        CONDITION_CALLBACK, ctypes.c_void_p, ctypes.c_void_p, ctypes.c_double, ctypes.c_char_p
    ]

    lib.zzstat_resolver_register_conditional_additive_transform.restype = ctypes.c_int32
    lib.zzstat_resolver_register_conditional_additive_transform.argtypes = [
        ctypes.c_void_p, ctypes.c_char_p, ctypes.c_ubyte, ctypes.c_ubyte,
        CONDITION_CALLBACK, ctypes.c_void_p, ctypes.c_void_p, ctypes.c_double, ctypes.c_char_p
    ]

    lib.zzstat_resolver_resolve.restype = ctypes.c_int32
    lib.zzstat_resolver_resolve.argtypes = [
        ctypes.c_void_p, ctypes.c_char_p, ctypes.c_void_p, ctypes.POINTER(ctypes.c_double)
    ]

    # Combat API
    RNG_CALLBACK = ctypes.CFUNCTYPE(ctypes.c_double, ctypes.c_void_p)
    lib.zzstat_combat_evaluate.restype = ctypes.c_int32
    lib.zzstat_combat_evaluate.argtypes = [
        ctypes.c_char_p, ctypes.c_void_p, ctypes.c_void_p, ctypes.c_void_p, ctypes.c_void_p,
        ctypes.c_void_p, ctypes.c_void_p, ctypes.POINTER(ctypes.c_double)
    ]


class StatContext:
    def __init__(self):
        if not lib:
            raise RuntimeError("zzstat shared library is not loaded")
        self.ptr = lib.zzstat_context_new()

    def __del__(self):
        if hasattr(self, 'ptr') and self.ptr and lib:
            lib.zzstat_context_free(self.ptr)
            self.ptr = None

    def set_float(self, key: str, value: float):
        lib.zzstat_context_set_float(self.ptr, key.encode('utf-8'), ctypes.c_double(value))

    def get_float(self, key: str, default_val: float = 0.0) -> float:
        return lib.zzstat_context_get_float(self.ptr, key.encode('utf-8'), ctypes.c_double(default_val))

    def set_bool(self, key: str, value: bool):
        lib.zzstat_context_set_bool(self.ptr, key.encode('utf-8'), ctypes.c_bool(value))

    def get_bool(self, key: str, default_val: bool = False) -> bool:
        return lib.zzstat_context_get_bool(self.ptr, key.encode('utf-8'), ctypes.c_bool(default_val))


class StatResolver:
    PHASE_ADDITIVE = 0
    PHASE_MULTIPLICATIVE = 1
    PHASE_FINAL = 2

    RULE_OVERRIDE = 0
    RULE_ADDITIVE = 1
    RULE_MULTIPLICATIVE = 2
    RULE_MIN = 3
    RULE_MAX = 4
    RULE_MIN_MAX = 5

    def __init__(self):
        if not lib:
            raise RuntimeError("zzstat shared library is not loaded")
        self.ptr = lib.zzstat_resolver_new()
        self._callbacks = []

    def __del__(self):
        if hasattr(self, 'ptr') and self.ptr and lib:
            lib.zzstat_resolver_free(self.ptr)
            self.ptr = None

    def register_constant_source(self, stat_id: str, value: float):
        lib.zzstat_resolver_register_constant_source(self.ptr, stat_id.encode('utf-8'), ctypes.c_double(value))

    def register_map_source(self, source: dict):
        if not source:
            return
        keys = list(source.keys())
        values = list(source.values())
        len_map = len(source)

        c_keys = (ctypes.c_char_p * len_map)(*[k.encode('utf-8') for k in keys])
        c_values = (ctypes.c_double * len_map)(*values)

        lib.zzstat_resolver_register_map_source(self.ptr, c_keys, c_values, len_map)

    def invalidate(self, stat_id: str):
        lib.zzstat_resolver_invalidate(self.ptr, stat_id.encode('utf-8'))

    def invalidate_all(self):
        lib.zzstat_resolver_invalidate_all(self.ptr)

    def register_additive_transform(self, stat_id: str, phase: int, rule: int, value: float):
        lib.zzstat_resolver_register_additive_transform(
            self.ptr, stat_id.encode('utf-8'), ctypes.c_ubyte(phase), ctypes.c_ubyte(rule), ctypes.c_double(value)
        )

    def register_multiplicative_transform(self, stat_id: str, phase: int, rule: int, value: float):
        lib.zzstat_resolver_register_multiplicative_transform(
            self.ptr, stat_id.encode('utf-8'), ctypes.c_ubyte(phase), ctypes.c_ubyte(rule), ctypes.c_double(value)
        )

    def register_clamp_transform(self, stat_id: str, phase: int, rule: int, has_min: bool, min_val: float, has_max: bool, max_val: float):
        lib.zzstat_resolver_register_clamp_transform(
            self.ptr, stat_id.encode('utf-8'), ctypes.c_ubyte(phase), ctypes.c_ubyte(rule),
            ctypes.c_bool(has_min), ctypes.c_double(min_val), ctypes.c_bool(has_max), ctypes.c_double(max_val)
        )

    def register_scaling_transform(self, stat_id: str, phase: int, rule: int, dependency: str, scale_factor: float):
        lib.zzstat_resolver_register_scaling_transform(
            self.ptr, stat_id.encode('utf-8'), ctypes.c_ubyte(phase), ctypes.c_ubyte(rule),
            dependency.encode('utf-8'), ctypes.c_double(scale_factor)
        )

    def register_conditional_multiplicative_transform(self, stat_id: str, phase: int, rule: int, condition, multiplier: float, description: str):
        def py_callback(ctx_ptr, user_data):
            ctx = StatContext()
            lib.zzstat_context_free(ctx.ptr)
            ctx.ptr = ctx_ptr
            res = bool(condition(ctx))
            ctx.ptr = None
            return res

        c_cb = CONDITION_CALLBACK(py_callback)
        self._callbacks.append(c_cb)

        lib.zzstat_resolver_register_conditional_multiplicative_transform(
            self.ptr, stat_id.encode('utf-8'), ctypes.c_ubyte(phase), ctypes.c_ubyte(rule),
            c_cb, None, None, ctypes.c_double(multiplier), description.encode('utf-8')
        )

    def register_conditional_additive_transform(self, stat_id: str, phase: int, rule: int, condition, bonus: float, description: str):
        def py_callback(ctx_ptr, user_data):
            ctx = StatContext()
            lib.zzstat_context_free(ctx.ptr)
            ctx.ptr = ctx_ptr
            res = bool(condition(ctx))
            ctx.ptr = None
            return res

        c_cb = CONDITION_CALLBACK(py_callback)
        self._callbacks.append(c_cb)

        lib.zzstat_resolver_register_conditional_additive_transform(
            self.ptr, stat_id.encode('utf-8'), ctypes.c_ubyte(phase), ctypes.c_ubyte(rule),
            c_cb, None, None, ctypes.c_double(bonus), description.encode('utf-8')
        )

    def resolve(self, stat_id: str, context: StatContext) -> float:
        out_val = ctypes.c_double(0.0)
        res = lib.zzstat_resolver_resolve(self.ptr, stat_id.encode('utf-8'), context.ptr, ctypes.byref(out_val))
        if res != 0:
            raise RuntimeError(f"Stat resolution failed with error code: {res}")
        return out_val.value


def evaluate_combat(formula_json: str, attacker: StatResolver, attacker_ctx: StatContext, defender: StatResolver, defender_ctx: StatContext, rng=None) -> float:
    if not lib:
        raise RuntimeError("zzstat shared library is not loaded")

    c_rng = None
    if rng:
        def py_rng(user_data):
            return float(rng())
        raw_cb = RNG_CALLBACK(py_rng)
        attacker._callbacks.append(raw_cb)
        c_rng = ctypes.cast(raw_cb, ctypes.c_void_p)

    out_result = ctypes.c_double(0.0)
    res = lib.zzstat_combat_evaluate(
        formula_json.encode('utf-8'), attacker.ptr, attacker_ctx.ptr, defender.ptr, defender_ctx.ptr,
        c_rng, None, ctypes.byref(out_result)
    )
    if res != 0:
        raise RuntimeError(f"Combat evaluation failed with error code: {res}")
    return out_result.value
