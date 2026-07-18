using System;
using System.Runtime.InteropServices;
using System.Text;

namespace Zzstat
{
    public class StatContext : IDisposable
    {
        internal IntPtr ptr;

        public StatContext()
        {
            ptr = Native.zzstat_context_new();
            if (ptr == IntPtr.Zero) throw new InvalidOperationException("Failed to create StatContext");
        }

        public void SetFloat(string key, double value)
        {
            Native.zzstat_context_set_float(ptr, Encoding.UTF8.GetBytes(key + "\0"), value);
        }

        public double GetFloat(string key, double defaultVal = 0.0)
        {
            return Native.zzstat_context_get_float(ptr, Encoding.UTF8.GetBytes(key + "\0"), defaultVal);
        }

        public void SetBool(string key, bool value)
        {
            Native.zzstat_context_set_bool(ptr, Encoding.UTF8.GetBytes(key + "\0"), value);
        }

        public bool GetBool(string key, bool defaultVal = false)
        {
            return Native.zzstat_context_get_bool(ptr, Encoding.UTF8.GetBytes(key + "\0"), defaultVal);
        }

        public void Dispose()
        {
            if (ptr != IntPtr.Zero)
            {
                Native.zzstat_context_free(ptr);
                ptr = IntPtr.Zero;
            }
        }

        ~StatContext() { Dispose(); }
    }

    public class StatResolver : IDisposable
    {
        internal IntPtr ptr;

        public const byte PHASE_ADDITIVE = 0;
        public const byte PHASE_MULTIPLICATIVE = 1;
        public const byte PHASE_FINAL = 2;

        public const byte RULE_OVERRIDE = 0;
        public const byte RULE_ADDITIVE = 1;
        public const byte RULE_MULTIPLICATIVE = 2;
        public const byte RULE_MIN = 3;
        public const byte RULE_MAX = 4;
        public const byte RULE_MIN_MAX = 5;

        public StatResolver()
        {
            ptr = Native.zzstat_resolver_new();
            if (ptr == IntPtr.Zero) throw new InvalidOperationException("Failed to create StatResolver");
        }

        public void RegisterConstantSource(string statId, double value)
        {
            Native.zzstat_resolver_register_constant_source(ptr, Encoding.UTF8.GetBytes(statId + "\0"), value);
        }

        public void Invalidate(string statId)
        {
            Native.zzstat_resolver_invalidate(ptr, Encoding.UTF8.GetBytes(statId + "\0"));
        }

        public void InvalidateAll()
        {
            Native.zzstat_resolver_invalidate_all(ptr);
        }

        public void RegisterAdditiveTransform(string statId, byte phase, byte rule, double value)
        {
            Native.zzstat_resolver_register_additive_transform(ptr, Encoding.UTF8.GetBytes(statId + "\0"), phase, rule, value);
        }

        public void RegisterMultiplicativeTransform(string statId, byte phase, byte rule, double value)
        {
            Native.zzstat_resolver_register_multiplicative_transform(ptr, Encoding.UTF8.GetBytes(statId + "\0"), phase, rule, value);
        }

        public void RegisterClampTransform(string statId, byte phase, byte rule, bool hasMin, double minVal, bool hasMax, double maxVal)
        {
            Native.zzstat_resolver_register_clamp_transform(ptr, Encoding.UTF8.GetBytes(statId + "\0"), phase, rule, hasMin, minVal, hasMax, maxVal);
        }

        public void RegisterScalingTransform(string statId, byte phase, byte rule, string dependency, double scaleFactor)
        {
            Native.zzstat_resolver_register_scaling_transform(ptr, Encoding.UTF8.GetBytes(statId + "\0"), phase, rule, Encoding.UTF8.GetBytes(dependency + "\0"), scaleFactor);
        }

        public double Resolve(string statId, StatContext context)
        {
            double outVal = 0;
            int res = Native.zzstat_resolver_resolve(ptr, Encoding.UTF8.GetBytes(statId + "\0"), context.ptr, ref outVal);
            if (res != 0)
            {
                throw new InvalidOperationException($"Stat resolution failed with error code: {res}");
            }
            return outVal;
        }

        public void Dispose()
        {
            if (ptr != IntPtr.Zero)
            {
                Native.zzstat_resolver_free(ptr);
                ptr = IntPtr.Zero;
            }
        }

        ~StatResolver() { Dispose(); }
    }

    public static class CombatEngine
    {
        public static double EvaluateCombat(string formulaJson, StatResolver attacker, StatContext attackerCtx, StatResolver defender, StatContext defenderCtx)
        {
            double outVal = 0;
            int res = Native.zzstat_combat_evaluate(
                Encoding.UTF8.GetBytes(formulaJson + "\0"),
                attacker.ptr, attackerCtx.ptr,
                defender.ptr, defenderCtx.ptr,
                IntPtr.Zero, IntPtr.Zero, ref outVal
            );
            if (res != 0)
            {
                throw new InvalidOperationException($"Combat evaluation failed with error code: {res}");
            }
            return outVal;
        }
    }

    internal static class Native
    {
        private const string LIB_NAME = "zzstat_ffi";

        [DllImport(LIB_NAME, CallingConvention = CallingConvention.Cdecl)]
        public static extern IntPtr zzstat_context_new();

        [DllImport(LIB_NAME, CallingConvention = CallingConvention.Cdecl)]
        public static extern void zzstat_context_free(IntPtr context);

        [DllImport(LIB_NAME, CallingConvention = CallingConvention.Cdecl)]
        public static extern int zzstat_context_set_float(IntPtr context, byte[] key, double value);

        [DllImport(LIB_NAME, CallingConvention = CallingConvention.Cdecl)]
        public static extern double zzstat_context_get_float(IntPtr context, byte[] key, double defaultVal);

        [DllImport(LIB_NAME, CallingConvention = CallingConvention.Cdecl)]
        public static extern int zzstat_context_set_bool(IntPtr context, byte[] key, bool value);

        [DllImport(LIB_NAME, CallingConvention = CallingConvention.Cdecl)]
        public static extern bool zzstat_context_get_bool(IntPtr context, byte[] key, bool defaultVal);

        [DllImport(LIB_NAME, CallingConvention = CallingConvention.Cdecl)]
        public static extern IntPtr zzstat_resolver_new();

        [DllImport(LIB_NAME, CallingConvention = CallingConvention.Cdecl)]
        public static extern void zzstat_resolver_free(IntPtr resolver);

        [DllImport(LIB_NAME, CallingConvention = CallingConvention.Cdecl)]
        public static extern int zzstat_resolver_register_constant_source(IntPtr resolver, byte[] statId, double value);

        [DllImport(LIB_NAME, CallingConvention = CallingConvention.Cdecl)]
        public static extern int zzstat_resolver_invalidate(IntPtr resolver, byte[] statId);

        [DllImport(LIB_NAME, CallingConvention = CallingConvention.Cdecl)]
        public static extern int zzstat_resolver_invalidate_all(IntPtr resolver);

        [DllImport(LIB_NAME, CallingConvention = CallingConvention.Cdecl)]
        public static extern int zzstat_resolver_register_additive_transform(IntPtr resolver, byte[] statId, byte phase, byte rule, double value);

        [DllImport(LIB_NAME, CallingConvention = CallingConvention.Cdecl)]
        public static extern int zzstat_resolver_register_multiplicative_transform(IntPtr resolver, byte[] statId, byte phase, byte rule, double value);

        [DllImport(LIB_NAME, CallingConvention = CallingConvention.Cdecl)]
        public static extern int zzstat_resolver_register_clamp_transform(IntPtr resolver, byte[] statId, byte phase, byte rule, bool hasMin, double minVal, bool hasMax, double maxVal);

        [DllImport(LIB_NAME, CallingConvention = CallingConvention.Cdecl)]
        public static extern int zzstat_resolver_register_scaling_transform(IntPtr resolver, byte[] statId, byte phase, byte rule, byte[] dependency, double scaleFactor);

        [DllImport(LIB_NAME, CallingConvention = CallingConvention.Cdecl)]
        public static extern int zzstat_resolver_resolve(IntPtr resolver, byte[] statId, IntPtr context, ref double outValue);

        [DllImport(LIB_NAME, CallingConvention = CallingConvention.Cdecl)]
        public static extern int zzstat_combat_evaluate(byte[] formulaJson, IntPtr attacker, IntPtr attackerCtx, IntPtr defender, IntPtr defenderCtx, IntPtr rngCallback, IntPtr rngUserData, ref double outResult);
    }
}
