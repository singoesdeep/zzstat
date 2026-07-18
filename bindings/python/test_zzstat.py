import unittest
import zzstat

class TestZzstat(unittest.TestCase):
    def test_basic_resolution(self):
        resolver = zzstat.StatResolver()
        ctx = zzstat.StatContext()

        # Register sources (additive)
        resolver.register_constant_source("HP", 100.0)
        resolver.register_constant_source("HP", 50.0)

        # Register transform
        resolver.register_multiplicative_transform("HP", zzstat.StatResolver.PHASE_MULTIPLICATIVE, zzstat.StatResolver.RULE_MULTIPLICATIVE, 1.5)

        # Resolve
        val = resolver.resolve("HP", ctx)
        self.assertEqual(val, 225.0)

    def test_scaling_and_conditional(self):
        resolver = zzstat.StatResolver()
        ctx = zzstat.StatContext()

        ctx.set_bool("in_combat", True)

        # Base ATK
        resolver.register_constant_source("ATK", 100.0)
        # Scaling from STR
        resolver.register_constant_source("STR", 10.0)
        resolver.register_scaling_transform("ATK", zzstat.StatResolver.PHASE_ADDITIVE, zzstat.StatResolver.RULE_ADDITIVE, "STR", 2.0)

        # Conditional transform
        resolver.register_conditional_multiplicative_transform(
            "ATK", zzstat.StatResolver.PHASE_MULTIPLICATIVE, zzstat.StatResolver.RULE_MULTIPLICATIVE,
            lambda c: c.get_bool("in_combat"), 1.5, "combat bonus"
        )

        val = resolver.resolve("ATK", ctx)
        self.assertEqual(val, 180.0)

        # Turn off combat
        ctx.set_bool("in_combat", False)
        resolver.invalidate("ATK")
        val = resolver.resolve("ATK", ctx)
        self.assertEqual(val, 120.0)

    def test_combat_evaluation(self):
        attacker = zzstat.StatResolver()
        attacker.register_constant_source("ATK", 150.0)

        defender = zzstat.StatResolver()
        defender.register_constant_source("DEF", 50.0)

        attacker_ctx = zzstat.StatContext()
        defender_ctx = zzstat.StatContext()

        formula_json = """{
            "name": "Basic Attack",
            "expression": {
                "type": "Clamp",
                "min": 0.0,
                "max": null,
                "expr": {
                    "type": "Subtract",
                    "left": { "type": "Stat", "target": "attacker", "stat": "ATK" },
                    "right": { "type": "Stat", "target": "defender", "stat": "DEF" }
                }
            }
        }"""

        damage = zzstat.evaluate_combat(formula_json, attacker, attacker_ctx, defender, defender_ctx)
        self.assertEqual(damage, 100.0)

if __name__ == '__main__':
    unittest.main()
