# zzstat - Detaylı API Dokümantasyonu

**zzstat** API dokümantasyonuna hoş geldin! Bu rehber, MMORPG veya standart RPG stat sistemleri kurmak için kütüphanenin temel modüllerini nasıl kullanacağını güncel ve detaylı kod örnekleriyle açıklamaktadır.

## İçindekiler
1. [Temel Konseptler](#1-temel-konseptler)
2. [StatResolver ve Kayıt İşlemleri](#2-statresolver-ve-kayıt-işlemleri)
3. [BonusAction API (Eşyalar ve Modifikatörler)](#3-bonusaction-api)
4. [ResourcePool (HP, Mana, DoT/HoT)](#4-resourcepool)
5. [StatusEffectManager (Bufflar, Debufflar ve Tetikleyiciler)](#5-statuseffectmanager)
6. [Combat Engine ve Bytecode VM](#6-combat-engine-ve-bytecode-vm)
7. [Hiyerarşik Çevre Sistemi (Environments)](#7-hiyerarşik-çevre-sistemi-environments)

---

## 1. Temel Konseptler

`zzstat` sisteminin temelinde üç ana yapı yer alır:
- `StatId`: Statı temsil eden benzersiz bir string sarmalayıcı (Örn: `"MAX_HP"`, `"STR"`, `"ATK"`). `StatId::from("STR")` ile oluşturulur.
- `StatValue`: Ham sayısal değer takma adıdır (Eğer `fixed-point` özelliği aktifse sabit noktalı sayı, aktif değilse `f64` olur).
- `StatContext`: Koşullu veya dinamik stat hesaplamalarında veri kontrolü sağlamak amacıyla kullanılan anahtar-değer depolama yapısıdır.

```rust
use zzstat::{StatId, StatContext};

let atk_id = StatId::from("ATK");
let def_id = StatId::from("DEF");

let mut ctx = StatContext::new();
ctx.set("STANCE", "DEFENSIVE");
```

---

## 2. StatResolver ve Kayıt İşlemleri

`StatResolver`, stat kaynaklarını (sources), dönüşümleri (transforms), bağımlılık grafiklerini yönetir ve hesaplama sonuçlarını önbelleğe (cache) alır.

### Örnek: Temel Stat ve Bağımlılık Tanımlama
```rust
use zzstat::resolver::StatResolver;
use zzstat::source::ConstantSource;
use zzstat::transform::standard::ScalingTransform;
use zzstat::{StatId, StatContext};

let mut resolver = StatResolver::new();

// 1. Karaktere 50 taban STR verelim
let str_id = StatId::from("STR");
resolver.register_source(str_id.clone(), Box::new(ConstantSource(50.0)));

// 2. ATK statını tanımlayalım. ATK'nın taban kaynağı yoktur, sadece STR'ye bağlıdır! (1 STR = 2 ATK)
let atk_id = StatId::from("ATK");
let str_to_atk = ScalingTransform::new(str_id.clone(), 2.0);
resolver.register_transform(atk_id.clone(), Box::new(str_to_atk));

// 3. Statları çözümlendirelim
let ctx = StatContext::new();
let resolved_str = resolver.resolve(&str_id, &ctx).unwrap(); // 50.0
let resolved_atk = resolver.resolve(&atk_id, &ctx).unwrap(); // 100.0 (50 * 2)

println!("STR: {}, ATK: {}", resolved_str.value.to_f64(), resolved_atk.value.to_f64());
```

---

## 3. BonusAction API

`Bonus` yapısı; ekipman statları, pasifler ve geçici bufflar için modifikatörleri akıcı bir builder deseniyle tanımlayıp derlemenizi (compile) sağlar.

### Kullanılabilir Metodlar:
- `Bonus::add(target).flat(value)`: Stat değerine düz ekleme yapar.
- `Bonus::scale(target, source).factor(value)`: Başka bir statı baz alarak hedef statı ölçeklendirir.
- `Bonus::mul(target).percent(value)`: Yüzdesel çarpan uygular (Örn: +%20 için `0.20`).
- `Bonus::r#override(target, value)`: Statı zorunlu olarak belirli bir değere eşitler.
- `Bonus::clamp_min(target, value)`: Statı minimum sınırda tutar.
- `Bonus::clamp_max(target, value)`: Statı maksimum sınırda tutar.

### Koşullu Bonuslar
Bonus tanımlamalarına `.with_condition(condition)` metodunu ekleyerek, bu modifikatörlerin yalnızca belirli şartlar altında (örn. defans duruşu) aktif olmasını sağlayabilirsiniz.

```rust
use zzstat::bonus::{Bonus, compile_bonus, apply_compiled_bonus};
use zzstat::transform::TransformPhase;
use zzstat::condition::ConditionDef;

// 1. Koşul tanımla (Yalnızca STANCE == DEFENSIVE durumunda aktif olsun)
let condition = ConditionDef::Equals {
    key: "STANCE".to_string(),
    value: serde_json::json!("DEFENSIVE"),
};

let bonus = Bonus::add(StatId::from("DEF"))
    .flat(50.0)
    .in_phase(TransformPhase::Additive)
    .with_condition(condition);

// 2. Derle ve Uygula
let compiled = compile_bonus::<f64>(&bonus);
let mut fork = resolver.fork();
apply_compiled_bonus(&mut fork, &compiled);
```

---

## 4. ResourcePool

`ResourcePool` yapısı; can (HP), mana (MP), enerji gibi sürekli değişen ve güncellenen dinamik havuz değerlerini yönetir. Değerleri otomatik olarak resolver'daki maksimum limite (örn: `MAX_HP`) bağlar ve sınırlandırır.

```rust
use zzstat::resource::{ResourcePool, TimeEffect, ThresholdTrigger, TriggerCondition};

// MAX_HP statına bağlı bir HP havuzu oluştur
let max_hp_id = StatId::from("MAX_HP");
let mut hp_pool = ResourcePool::new(max_hp_id.clone());

// Başlangıçta canı doldur
hp_pool.fill(&resolver, &ctx);

// Karakter üzerine Zehir etkisi (DoT) ekle (3 tick boyunca tick başına -20 HP)
hp_pool.add_effect(TimeEffect {
    name: "Poison".to_string(),
    amount_per_tick: -20.0,
    ticks_remaining: 3,
});

// Can tükendiğinde tetiklenecek "Ölüm" olayı kaydet
hp_pool.add_trigger(ThresholdTrigger {
    condition: TriggerCondition::Empty,
    event_name: "DEATH".to_string(),
});

// Oyun döngüsü
for _ in 0..3 {
    let events = hp_pool.tick(&resolver, &ctx);
    println!("Mevcut HP: {}", hp_pool.current_value());
    
    if events.contains(&"DEATH".to_string()) {
        println!("Karakter öldü!");
    }
}
```

---

## 5. StatusEffectManager

`StatusEffectManager`, karakter üzerindeki geçici etkileri yönetir ve `O(1)` kopyalama maliyetiyle çalışan copy-on-write `fork()` mekanizmasını kullanır. Taban statları değiştirmeden modifikasyonları karakterin üzerine bindirir.

### Reaktif Durum Tetikleyicileri (Effect Triggers)
Oyunda belirli bir olay (event) gerçekleştiğinde (örn: karakterin hasar alması), belirli koşullar altında bir durum etkisinin otomatik olarak tetiklenmesini sağlayabilirsiniz.

```rust
use zzstat::status_effect::{StatusEffectManager, StatusEffect, EffectTrigger, StackBehavior};
use zzstat::bonus::Bonus;
use zzstat::transform::TransformPhase;

let mut manager = StatusEffectManager::new();

// 1. Savaş Çığlığı etkisi tanımla (+50 ATK)
let warcry = StatusEffect {
    id: "WARCRY".to_string(),
    name: "Warcry".to_string(),
    bonuses: vec![Bonus::add(StatId::from("ATK"))
        .flat(50.0)
        .in_phase(TransformPhase::Additive)],
    max_stacks: 1,
    stack_behavior: StackBehavior::Refresh,
};

// 2. Tetikleyici kaydet: "on_combat_start" olayı oluştuğunda bu etkiyi uygula
let trigger = EffectTrigger {
    event: "on_combat_start".to_string(),
    condition: None,
    effect: warcry,
    duration_ticks: Some(5),
    stacks: 1,
};
manager.register_trigger(trigger);

// 3. Olayı tetikle
let ctx = StatContext::new();
manager.fire_event("on_combat_start", &ctx);

// 4. Etkin statları hesaplamak için aktif resolver'ı al
let mut active_resolver = manager.get_active_resolver(&base_resolver);
let resolved = active_resolver.resolve(&StatId::from("ATK"), &ctx).unwrap();
```

---

## 6. Combat Engine ve Bytecode VM

`CombatEngine`, savaş hasar formüllerini işletir. Formülleri AST ağacında recursive çözebileceğiniz gibi, düz bytecode (`Opcode` listesi) formatına önceden derleyip (pre-compile) yığın tabanlı **sanal makinede (VM)** maksimum performansla çalıştırabilirsiniz.

### Bytecode VM Derleme ve Çalıştırma
```rust
use zzstat::combat::{CombatEngine, CombatFormula, CombatExpression, CombatTarget};

// 1. Hasar formülünü tanımla (genellikle bir JSON dosyasından okunur)
let formula = CombatFormula {
    name: "Crit Hit".to_string(),
    expression: CombatExpression::Multiply {
        left: Box::new(CombatExpression::Stat {
            target: CombatTarget::Attacker,
            stat: "ATK".to_string(),
        }),
        right: Box::new(CombatExpression::Constant { value: 2.0 }),
    },
};

// 2. Formülü düz bytecode'a derle
let compiled = formula.compile();

// 3. Bytecode'u VM üzerinde çalıştır
let mut rng = || 0.5; // Şans hesaplamaları için RNG
let damage = CombatEngine::evaluate_compiled(
    &compiled,
    &mut attacker_resolver,
    &attacker_ctx,
    &mut defender_resolver,
    &defender_ctx,
    &mut rng,
).unwrap();

println!("Hedefe {} hasar verildi!", damage);
```

---

## 7. Hiyerarşik Çevre Sistemi (Environments)

Çevre sistemi, birden fazla resolver'ı parent-child (ebeveyn-çocuk) ilişkisiyle birbirine bağlamanıza (`Weather -> Zone -> Party -> Character`) imkan tanır.

Üst katmanlara eklenen tüm stat modifikasyonları alt katmanlar tarafından otomatik olarak miras alınır. Üst katmanlarda çalışma anında dinamik bir değişiklik yapıldığında (örn: hava durumunun değişmesi vb.), alt katmanlardaki çözümlenen statlar da anında güncellenir.

```rust
// 1. Hava Durumu Resolver (Hiyerarşinin en tepesi)
let mut weather = StatResolver::new();
weather.register_source(StatId::from("HP"), Box::new(ConstantSource(100.0)));

// 2. Bölge Resolver (Hava durumundan çatallandı)
let mut zone = weather.fork();
zone.register_transform(StatId::from("ATK"), Box::new(MultiplicativeTransform::new(1.2)));

// 3. Karakter Resolver (Bölgeden çatallandı)
let mut character = zone.fork();
character.register_source(StatId::from("HP"), Box::new(ConstantSource(50.0)));

// Statları çözümle (Üst düğümlerdeki tüm etkileri miras alır!)
let ctx = StatContext::new();
let resolved_hp = character.resolve(&StatId::from("HP"), &ctx).unwrap(); // 100 + 50 = 150
```
