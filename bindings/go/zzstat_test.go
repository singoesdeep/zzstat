package zzstat_test

import (
	"testing"
	"github.com/singoesdeep/zzstat/bindings/go"
)

func TestBasicResolution(t *testing.T) {
	resolver := zzstat.NewResolver()
	defer resolver.Free()

	ctx := zzstat.NewContext()
	defer ctx.Free()

	// Register sources (additive)
	resolver.RegisterConstantSource("HP", 100.0)
	resolver.RegisterConstantSource("HP", 50.0)

	// Register transform
	resolver.RegisterMultiplicativeTransform("HP", zzstat.PhaseMultiplicative, zzstat.RuleMultiplicative, 1.5)

	// Resolve
	val, err := resolver.Resolve("HP", ctx)
	if err != nil {
		t.Fatalf("Failed to resolve HP: %v", err)
	}

	expected := 225.0 // (100 + 50) * 1.5
	if val != expected {
		t.Errorf("Expected HP to be %f, got %f", expected, val)
	}
}

func TestScalingAndConditional(t *testing.T) {
	resolver := zzstat.NewResolver()
	defer resolver.Free()

	ctx := zzstat.NewContext()
	defer ctx.Free()

	ctx.SetBool("in_combat", true)

	// Base ATK
	resolver.RegisterConstantSource("ATK", 100.0)
	// Scaling from STR (ATK += STR * 2.0)
	resolver.RegisterConstantSource("STR", 10.0)
	resolver.RegisterScalingTransform("ATK", zzstat.PhaseAdditive, zzstat.RuleAdditive, "STR", 2.0)

	// Conditional transform: only if in_combat is true, multiply ATK by 1.5
	resolver.RegisterConditionalMultiplicativeTransform("ATK", zzstat.PhaseMultiplicative, zzstat.RuleMultiplicative, func(c *zzstat.Context) bool {
		return c.GetBool("in_combat", false)
	}, 1.5, "combat bonus")

	val, err := resolver.Resolve("ATK", ctx)
	if err != nil {
		t.Fatalf("Failed to resolve ATK: %v", err)
	}

	expected := 180.0 // (100 + 10 * 2) * 1.5
	if val != expected {
		t.Errorf("Expected ATK to be %f, got %f", expected, val)
	}

	// Turn off combat
	ctx.SetBool("in_combat", false)
	resolver.Invalidate("ATK")
	val, err = resolver.Resolve("ATK", ctx)
	if err != nil {
		t.Fatalf("Failed to resolve ATK: %v", err)
	}

	expected = 120.0 // (100 + 10 * 2) without 1.5 multiplier
	if val != expected {
		t.Errorf("Expected ATK (out of combat) to be %f, got %f", expected, val)
	}
}

func TestCombatEvaluation(t *testing.T) {
	attacker := zzstat.NewResolver()
	defer attacker.Free()
	attacker.RegisterConstantSource("ATK", 150.0)

	defender := zzstat.NewResolver()
	defer defender.Free()
	defender.RegisterConstantSource("DEF", 50.0)

	attackerCtx := zzstat.NewContext()
	defer attackerCtx.Free()

	defenderCtx := zzstat.NewContext()
	defer defenderCtx.Free()

	formulaJSON := `{
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
	}`

	damage, err := zzstat.EvaluateCombat(formulaJSON, attacker, attackerCtx, defender, defenderCtx, nil)
	if err != nil {
		t.Fatalf("Failed to evaluate combat: %v", err)
	}

	expected := 100.0 // 150 - 50
	if damage != expected {
		t.Errorf("Expected damage to be %f, got %f", expected, damage)
	}
}
