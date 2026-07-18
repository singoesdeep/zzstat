use std::ffi::{c_char, c_void, CStr};
use std::panic::catch_unwind;
use zzstat::combat::{CombatEngine, CombatFormula};
use zzstat::{
    AdditiveTransform, ClampTransform, ConstantSource, MultiplicativeTransform,
    ScalingTransform, StackRule, StatContext, StatError, StatId, StatNumeric, StatResolver,
    StatValue, TransformPhase, ConditionalTransform,
};

// Error codes
pub const ZZSTAT_SUCCESS: i32 = 0;
pub const ZZSTAT_ERROR_CYCLE_DETECTED: i32 = 1;
pub const ZZSTAT_ERROR_MISSING_DEPENDENCY: i32 = 2;
pub const ZZSTAT_ERROR_INVALID_STAT: i32 = 3;
pub const ZZSTAT_ERROR_VM_ERROR: i32 = 4;
pub const ZZSTAT_ERROR_PANIC: i32 = 5;
pub const ZZSTAT_ERROR_NULL_POINTER: i32 = 6;
pub const ZZSTAT_ERROR_JSON_ERROR: i32 = 7;
pub const ZZSTAT_ERROR_UNKNOWN: i32 = 8;

fn map_error(err: &StatError) -> i32 {
    match err {
        StatError::Cycle { .. } => ZZSTAT_ERROR_CYCLE_DETECTED,
        StatError::MissingDependency(_) => ZZSTAT_ERROR_MISSING_DEPENDENCY,
        StatError::MissingSource(_) => ZZSTAT_ERROR_INVALID_STAT,
        StatError::InvalidTransform(_, _) => ZZSTAT_ERROR_INVALID_STAT,
        StatError::VmError(_) => ZZSTAT_ERROR_VM_ERROR,
    }
}

// Helper to convert C string to &str
unsafe fn c_str_to_str<'a>(ptr: *const c_char) -> Option<&'a str> {
    if ptr.is_null() {
        return None;
    }
    CStr::from_ptr(ptr).to_str().ok()
}

// --- Context API ---

#[no_mangle]
pub extern "C" fn zzstat_context_new() -> *mut StatContext {
    Box::into_raw(Box::new(StatContext::new()))
}

#[no_mangle]
pub unsafe extern "C" fn zzstat_context_free(context: *mut StatContext) {
    if !context.is_null() {
        let _ = Box::from_raw(context);
    }
}

#[no_mangle]
pub unsafe extern "C" fn zzstat_context_set_float(
    context: *mut StatContext,
    key: *const c_char,
    value: f64,
) -> i32 {
    if context.is_null() {
        return ZZSTAT_ERROR_NULL_POINTER;
    }
    let context = &mut *context;
    let key_str = match c_str_to_str(key) {
        Some(s) => s,
        None => return ZZSTAT_ERROR_NULL_POINTER,
    };
    context.set(key_str, value);
    ZZSTAT_SUCCESS
}

#[no_mangle]
pub unsafe extern "C" fn zzstat_context_get_float(
    context: *const StatContext,
    key: *const c_char,
    default_val: f64,
) -> f64 {
    if context.is_null() {
        return default_val;
    }
    let context = &*context;
    let key_str = match c_str_to_str(key) {
        Some(s) => s,
        None => return default_val,
    };
    context.get::<f64>(key_str).unwrap_or(default_val)
}

#[no_mangle]
pub unsafe extern "C" fn zzstat_context_set_bool(
    context: *mut StatContext,
    key: *const c_char,
    value: bool,
) -> i32 {
    if context.is_null() {
        return ZZSTAT_ERROR_NULL_POINTER;
    }
    let context = &mut *context;
    let key_str = match c_str_to_str(key) {
        Some(s) => s,
        None => return ZZSTAT_ERROR_NULL_POINTER,
    };
    context.set(key_str, value);
    ZZSTAT_SUCCESS
}

#[no_mangle]
pub unsafe extern "C" fn zzstat_context_get_bool(
    context: *const StatContext,
    key: *const c_char,
    default_val: bool,
) -> bool {
    if context.is_null() {
        return default_val;
    }
    let context = &*context;
    let key_str = match c_str_to_str(key) {
        Some(s) => s,
        None => return default_val,
    };
    context.get::<bool>(key_str).unwrap_or(default_val)
}

// --- Resolver API ---

#[no_mangle]
pub extern "C" fn zzstat_resolver_new() -> *mut StatResolver {
    Box::into_raw(Box::new(StatResolver::new()))
}

#[no_mangle]
pub unsafe extern "C" fn zzstat_resolver_free(resolver: *mut StatResolver) {
    if !resolver.is_null() {
        let _ = Box::from_raw(resolver);
    }
}

#[no_mangle]
pub unsafe extern "C" fn zzstat_resolver_register_constant_source(
    resolver: *mut StatResolver,
    stat_id: *const c_char,
    value: f64,
) -> i32 {
    if resolver.is_null() {
        return ZZSTAT_ERROR_NULL_POINTER;
    }
    let resolver = &mut *resolver;
    let stat_str = match c_str_to_str(stat_id) {
        Some(s) => s,
        None => return ZZSTAT_ERROR_NULL_POINTER,
    };
    resolver.register_source(StatId::from(stat_str), Box::new(ConstantSource(value)));
    ZZSTAT_SUCCESS
}

#[no_mangle]
pub unsafe extern "C" fn zzstat_resolver_register_map_source(
    resolver: *mut StatResolver,
    keys: *const *const c_char,
    values: *const f64,
    len: usize,
) -> i32 {
    if resolver.is_null() || keys.is_null() || values.is_null() {
        return ZZSTAT_ERROR_NULL_POINTER;
    }
    let resolver = &mut *resolver;
    let keys_slice = std::slice::from_raw_parts(keys, len);
    let values_slice = std::slice::from_raw_parts(values, len);

    for i in 0..len {
        let stat_str = match c_str_to_str(keys_slice[i]) {
            Some(s) => s,
            None => return ZZSTAT_ERROR_NULL_POINTER,
        };
        resolver.register_source(StatId::from(stat_str), Box::new(ConstantSource(values_slice[i])));
    }
    ZZSTAT_SUCCESS
}

#[no_mangle]
pub unsafe extern "C" fn zzstat_resolver_invalidate(
    resolver: *mut StatResolver,
    stat_id: *const c_char,
) -> i32 {
    if resolver.is_null() {
        return ZZSTAT_ERROR_NULL_POINTER;
    }
    let resolver = &mut *resolver;
    let stat_str = match c_str_to_str(stat_id) {
        Some(s) => s,
        None => return ZZSTAT_ERROR_NULL_POINTER,
    };
    resolver.invalidate(&StatId::from(stat_str));
    ZZSTAT_SUCCESS
}

#[no_mangle]
pub unsafe extern "C" fn zzstat_resolver_invalidate_all(
    resolver: *mut StatResolver,
) -> i32 {
    if resolver.is_null() {
        return ZZSTAT_ERROR_NULL_POINTER;
    }
    let resolver = &mut *resolver;
    resolver.invalidate_all();
    ZZSTAT_SUCCESS
}

fn map_phase(phase: u8) -> TransformPhase {
    match phase {
        0 => TransformPhase::Additive,
        1 => TransformPhase::Multiplicative,
        2 => TransformPhase::Final,
        custom => TransformPhase::Custom(custom),
    }
}

fn map_rule(rule: u8) -> StackRule {
    match rule {
        0 => StackRule::Override,
        1 => StackRule::Additive,
        2 => StackRule::Multiplicative,
        3 => StackRule::Min,
        4 => StackRule::Max,
        5 => StackRule::MinMax,
        _ => StackRule::Additive,
    }
}

#[no_mangle]
pub unsafe extern "C" fn zzstat_resolver_register_additive_transform(
    resolver: *mut StatResolver,
    stat_id: *const c_char,
    phase: u8,
    rule: u8,
    value: f64,
) -> i32 {
    if resolver.is_null() {
        return ZZSTAT_ERROR_NULL_POINTER;
    }
    let resolver = &mut *resolver;
    let stat_str = match c_str_to_str(stat_id) {
        Some(s) => s,
        None => return ZZSTAT_ERROR_NULL_POINTER,
    };
    resolver.register_transform_with_rule(
        StatId::from(stat_str),
        map_phase(phase),
        map_rule(rule),
        Box::new(AdditiveTransform::new(value)),
    );
    ZZSTAT_SUCCESS
}

#[no_mangle]
pub unsafe extern "C" fn zzstat_resolver_register_multiplicative_transform(
    resolver: *mut StatResolver,
    stat_id: *const c_char,
    phase: u8,
    rule: u8,
    value: f64,
) -> i32 {
    if resolver.is_null() {
        return ZZSTAT_ERROR_NULL_POINTER;
    }
    let resolver = &mut *resolver;
    let stat_str = match c_str_to_str(stat_id) {
        Some(s) => s,
        None => return ZZSTAT_ERROR_NULL_POINTER,
    };
    resolver.register_transform_with_rule(
        StatId::from(stat_str),
        map_phase(phase),
        map_rule(rule),
        Box::new(MultiplicativeTransform::new(value)),
    );
    ZZSTAT_SUCCESS
}

#[no_mangle]
pub unsafe extern "C" fn zzstat_resolver_register_clamp_transform(
    resolver: *mut StatResolver,
    stat_id: *const c_char,
    phase: u8,
    rule: u8,
    has_min: bool,
    min_val: f64,
    has_max: bool,
    max_val: f64,
) -> i32 {
    if resolver.is_null() {
        return ZZSTAT_ERROR_NULL_POINTER;
    }
    let resolver = &mut *resolver;
    let stat_str = match c_str_to_str(stat_id) {
        Some(s) => s,
        None => return ZZSTAT_ERROR_NULL_POINTER,
    };
    let min_opt = if has_min { Some(StatValue::from_f64(min_val)) } else { None };
    let max_opt = if has_max { Some(StatValue::from_f64(max_val)) } else { None };
    resolver.register_transform_with_rule(
        StatId::from(stat_str),
        map_phase(phase),
        map_rule(rule),
        Box::new(ClampTransform::with_bounds(min_opt, max_opt)),
    );
    ZZSTAT_SUCCESS
}

#[no_mangle]
pub unsafe extern "C" fn zzstat_resolver_register_scaling_transform(
    resolver: *mut StatResolver,
    stat_id: *const c_char,
    phase: u8,
    rule: u8,
    dependency: *const c_char,
    scale_factor: f64,
) -> i32 {
    if resolver.is_null() {
        return ZZSTAT_ERROR_NULL_POINTER;
    }
    let resolver = &mut *resolver;
    let stat_str = match c_str_to_str(stat_id) {
        Some(s) => s,
        None => return ZZSTAT_ERROR_NULL_POINTER,
    };
    let dep_str = match c_str_to_str(dependency) {
        Some(s) => s,
        None => return ZZSTAT_ERROR_NULL_POINTER,
    };
    resolver.register_transform_with_rule(
        StatId::from(stat_str),
        map_phase(phase),
        map_rule(rule),
        Box::new(ScalingTransform::new(StatId::from(dep_str), scale_factor)),
    );
    ZZSTAT_SUCCESS
}

pub type ConditionCallback = unsafe extern "C" fn(ctx: *const StatContext, user_data: *mut c_void) -> bool;
pub type FreeUserDataCallback = unsafe extern "C" fn(user_data: *mut c_void);

struct FfiCondition {
    callback: ConditionCallback,
    user_data: *mut c_void,
    free_user_data: Option<FreeUserDataCallback>,
}

unsafe impl Send for FfiCondition {}
unsafe impl Sync for FfiCondition {}

impl Drop for FfiCondition {
    fn drop(&mut self) {
        if let Some(free_fn) = self.free_user_data {
            unsafe {
                free_fn(self.user_data);
            }
        }
    }
}

impl FfiCondition {
    fn evaluate(&self, ctx: &StatContext) -> bool {
        unsafe {
            (self.callback)(ctx as *const StatContext, self.user_data)
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn zzstat_resolver_register_conditional_multiplicative_transform(
    resolver: *mut StatResolver,
    stat_id: *const c_char,
    phase: u8,
    rule: u8,
    callback: ConditionCallback,
    user_data: *mut c_void,
    free_user_data: Option<FreeUserDataCallback>,
    multiplier: f64,
    description: *const c_char,
) -> i32 {
    if resolver.is_null() {
        return ZZSTAT_ERROR_NULL_POINTER;
    }
    let resolver = &mut *resolver;
    let stat_str = match c_str_to_str(stat_id) {
        Some(s) => s,
        None => return ZZSTAT_ERROR_NULL_POINTER,
    };
    let desc_str = match c_str_to_str(description) {
        Some(s) => s,
        None => "conditional multiplicative",
    };

    let ffi_cond = FfiCondition {
        callback,
        user_data,
        free_user_data,
    };

    let inner = Box::new(MultiplicativeTransform::new(multiplier));
    let cond_transform = ConditionalTransform::new(
        move |ctx| ffi_cond.evaluate(ctx),
        inner,
        desc_str,
    );

    resolver.register_transform_with_rule(
        StatId::from(stat_str),
        map_phase(phase),
        map_rule(rule),
        Box::new(cond_transform),
    );
    ZZSTAT_SUCCESS
}

#[no_mangle]
pub unsafe extern "C" fn zzstat_resolver_register_conditional_additive_transform(
    resolver: *mut StatResolver,
    stat_id: *const c_char,
    phase: u8,
    rule: u8,
    callback: ConditionCallback,
    user_data: *mut c_void,
    free_user_data: Option<FreeUserDataCallback>,
    bonus: f64,
    description: *const c_char,
) -> i32 {
    if resolver.is_null() {
        return ZZSTAT_ERROR_NULL_POINTER;
    }
    let resolver = &mut *resolver;
    let stat_str = match c_str_to_str(stat_id) {
        Some(s) => s,
        None => return ZZSTAT_ERROR_NULL_POINTER,
    };
    let desc_str = match c_str_to_str(description) {
        Some(s) => s,
        None => "conditional additive",
    };

    let ffi_cond = FfiCondition {
        callback,
        user_data,
        free_user_data,
    };

    let inner = Box::new(AdditiveTransform::new(bonus));
    let cond_transform = ConditionalTransform::new(
        move |ctx| ffi_cond.evaluate(ctx),
        inner,
        desc_str,
    );

    resolver.register_transform_with_rule(
        StatId::from(stat_str),
        map_phase(phase),
        map_rule(rule),
        Box::new(cond_transform),
    );
    ZZSTAT_SUCCESS
}

#[no_mangle]
pub unsafe extern "C" fn zzstat_resolver_resolve(
    resolver: *mut StatResolver,
    stat_id: *const c_char,
    context: *const StatContext,
    out_value: *mut f64,
) -> i32 {
    if resolver.is_null() || context.is_null() || out_value.is_null() {
        return ZZSTAT_ERROR_NULL_POINTER;
    }
    let resolver = &mut *resolver;
    let context = &*context;
    let stat_str = match c_str_to_str(stat_id) {
        Some(s) => s,
        None => return ZZSTAT_ERROR_NULL_POINTER,
    };

    let res = catch_unwind(std::panic::AssertUnwindSafe(|| {
        resolver.resolve(&StatId::from(stat_str), context)
    }));

    match res {
        Ok(Ok(resolved)) => {
            *out_value = resolved.value.to_f64();
            ZZSTAT_SUCCESS
        }
        Ok(Err(err)) => map_error(&err),
        Err(_) => ZZSTAT_ERROR_PANIC,
    }
}

// --- Combat API ---

#[no_mangle]
pub unsafe extern "C" fn zzstat_combat_evaluate(
    formula_json: *const c_char,
    attacker_resolver: *mut StatResolver,
    attacker_ctx: *const StatContext,
    defender_resolver: *mut StatResolver,
    defender_ctx: *const StatContext,
    rng_callback: Option<unsafe extern "C" fn(user_data: *mut c_void) -> f64>,
    rng_user_data: *mut c_void,
    out_result: *mut f64,
) -> i32 {
    if formula_json.is_null() || attacker_resolver.is_null() || attacker_ctx.is_null() ||
       defender_resolver.is_null() || defender_ctx.is_null() || out_result.is_null() {
        return ZZSTAT_ERROR_NULL_POINTER;
    }

    let formula_str = match c_str_to_str(formula_json) {
        Some(s) => s,
        None => return ZZSTAT_ERROR_NULL_POINTER,
    };

    let formula: CombatFormula = match serde_json::from_str(formula_str) {
        Ok(f) => f,
        Err(_) => return ZZSTAT_ERROR_JSON_ERROR,
    };

    let attacker_resolver = &mut *attacker_resolver;
    let attacker_ctx = &*attacker_ctx;
    let defender_resolver = &mut *defender_resolver;
    let defender_ctx = &*defender_ctx;

    let res = catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut rng = move || {
            if let Some(cb) = rng_callback {
                cb(rng_user_data)
            } else {
                0.0
            }
        };
        CombatEngine::evaluate(
            &formula,
            attacker_resolver,
            attacker_ctx,
            defender_resolver,
            defender_ctx,
            &mut rng,
        )
    }));

    match res {
        Ok(Ok(val)) => {
            *out_result = val;
            ZZSTAT_SUCCESS
        }
        Ok(Err(err)) => map_error(&err),
        Err(_) => ZZSTAT_ERROR_PANIC,
    }
}
