# zzstat - Detaylı API Dokümantasyonu

**zzstat** API dokümantasyonuna hoş geldin! Bu rehber, MMORPG veya standart RPG stat sistemleri kurmak için oyun motorunun temel modüllerini nasıl kullanacağını detaylı örneklerle açıklamaktadır.

## İçindekiler
1. [Temel Konseptler](#1-temel-konseptler)
2. [StatRegistry ve StatResolver](#2-statregistry-ve-statresolver)
3. [BonusAction API (Eşyalar ve Modifierlar)](#3-bonusaction-api)
4. [ResourcePool (HP, Mana ve Sınırlandırmalar)](#4-resourcepool)
5. [StatusManager (Geçici Buff ve Debufflar)](#5-statusmanager)
6. [Combat Engine (Savaş Motoru ve Formüller)](#6-combat-engine)

---

## 1. Temel Konseptler

`zzstat`'ın kalbinde üç ana veri tipi yatar:
- `StatId`: Bir statı temsil eden benzersiz string sarmalayıcı (Örn: `"MAX_HP"`, `"STR"`, `"ATK"`).
- `StatValue`: Ham sayıyı temsil eden bir `f64` takma adıdır (Eğer `fixed-point` aktifleştirilirse değişir).
- `StatContext`: Gelecekte çevre değişkenleri vb. bağlamsal verileri taşımak için hazırlanmış boş bir struct.

```rust
use zzstat::{StatId, StatValue, StatContext};

let atk_id = StatId::new("ATK");
let def_id = StatId::new("DEF");
let ctx = StatContext;
```

---

## 2. StatRegistry ve StatResolver

Motorumuz mantık (logic) ve veri (data) mimarisini birbirinden ayırır.
- **`StatRegistry`**: Ham tanımlamaları (Base kaynakları, Çarpanları ve Bağımlılıkları) depolar.
- **`StatResolver`**: Registry'i sarar ve matematiği işletir. Bağımlılıkları dinamik çözer ve hesaplama sonuçlarını önbelleğe (cache) alır.

### Örnek: Base Stat ve Bağımlılık Tanımlamak
```rust
use zzstat::registry::StatRegistry;
use zzstat::resolver::StatResolver;
use zzstat::source::ConstantSource;
use zzstat::transform::standard::ScalingTransform;
use zzstat::transform::core::{TransformEntry, StackRule, TransformPhase};
use zzstat::{StatId, StatContext};
use std::sync::Arc;

let mut registry = StatRegistry::new();

// 1. Karaktere temel olarak 50 STR verelim.
let str_id = StatId::new("STR");
registry.add_source(str_id.clone(), Box::new(ConstantSource::new(50.0)));

// 2. ATK statını tanımlayalım. Ancak ATK'nın base değeri yok, sadece STR'ye bağlı!
let atk_id = StatId::new("ATK");
let str_to_atk = ScalingTransform::new(str_id.clone(), 2.0); // 1 STR = 2 ATK kazandırır

registry.add_transform(
    atk_id.clone(),
    TransformEntry::new(
        TransformPhase::Base, 
        StackRule::Additive, 
        Box::new(str_to_atk)
    )
);

// 3. Statları Resolve Edip Çözelim
let mut resolver = StatResolver::new(registry);
let resolved_str = resolver.resolve(&str_id, &StatContext); // 50.0 döner
let resolved_atk = resolver.resolve(&atk_id, &StatContext); // 100.0 döner (50 * 2)

println!("STR: {}, ATK: {}", resolved_str, resolved_atk);
```

---

## 3. BonusAction API

`BonusAction` enum'u, eşya ve item özelliklerini yönetmenin en güvenli ve rusty (idiomatic) yoludur. Karmaşık `TransformEntry` veri yapılarıyla uğraşmak yerine, çok daha okunabilir yardımcı metodlarla bonusları yaratıp (compile) sisteme enjekte edebilirsiniz.

### Mevcut Metodlar:
- `Bonus::add_flat()`: Düz bir değer ekler (+50 HP).
- `Bonus::scale()`: Başka bir stat üzerinden yüzde alır (+%50 STR kazancı).
- `Bonus::multiply()`: Statın kendi taban değeriyle çarpar (+%20 Genel ATK).
- `Bonus::override_value()`: Statı zorunlu olarak belirli bir değere eşitler (Tüm çarpanları ezer).

### Örnek: Bir Kılıç Giymek
```rust
use zzstat::bonus::{Bonus, apply_compiled_bonus, compile_bonus};

// Bir eşyanın statlarını tanımlayalım
let sword_bonuses = vec![
    Bonus::add_flat(StatId::new("ATK"), 120.0),      // +120 Düz ATK
    Bonus::multiply(StatId::new("ATK"), 0.15),       // +%15 Toplam ATK
];

// Bonusları derleyip karaktere (resolver'a) uygulayalım
for bonus in sword_bonuses {
    let compiled = compile_bonus::<f64>(&bonus);
    apply_compiled_bonus(&mut resolver, &compiled);
}

// Artık resolver.resolve(ATK) çağrıldığında yeni bonuslar hesaba katılacaktır!
```

---

## 4. ResourcePool

`ResourcePool`; HP, MP, Enerji gibi sürekli değişebilen (stateful) hayati değerleri yönetir. Bu değerleri resolver'daki bir üst sınıra (örn: `MAX_HP`) otomatik bağlar. Zırh çıkarıldığında maksimum stat düşerse anında canı da yeni limite kırpar (clamp).

```rust
use zzstat::resource::{ResourcePool, TimeEffect, ThresholdTrigger, TriggerCondition};

// MAX_HP statına bağlı bir HP havuzu yaratalım
let max_hp_id = StatId::new("MAX_HP");
let mut hp_pool = ResourcePool::new(max_hp_id.clone());

// Karakterin canını tamamen dolduralım (100 / 100)
hp_pool.fill(&resolver, &ctx);

// Zehir etkisini (DoT) uygulayalım (3 tur boyunca saniyede -20 HP)
hp_pool.add_effect(TimeEffect {
    name: "Poison".to_string(),
    amount_per_tick: -20.0,
    ticks_remaining: 3,
});

// Can 0'a ulaştığında fırlatılacak bir Death trigger'ı ekleyelim
hp_pool.add_trigger(ThresholdTrigger {
    condition: TriggerCondition::Empty,
    event_name: "DEATH".to_string(),
});

// Oyun döngüsü:
for _ in 0..3 {
    let events = hp_pool.tick(&resolver, &ctx);
    println!("Mevcut Can (HP): {}", hp_pool.current_value());
    
    if events.contains(&"DEATH".to_string()) {
        println!("Karakter Öldü!");
    }
}
```

---

## 5. StatusManager

`StatusManager`, `O(1)` bellek tahsisatı yapan devasa hızdaki copy-on-write `fork()` mekanizmasını kullanır. Bir karakterin taban statlarını ASLA bozmadan üzerine istediğiniz kadar geçici buff (iksir, büyü vs.) ekleyebilirsiniz.

```rust
use zzstat::status::{StatusManager, StatusEffect, StackBehavior};

let mut status_manager = StatusManager::new();

// Bir Savaş Çığlığı (Buff) Yaratalım
let warcry = StatusEffect {
    id: "WARCRY".to_string(),
    name: "Warcry (+50 ATK)".to_string(),
    bonuses: vec![Bonus::add_flat(StatId::new("ATK"), 50.0)],
    max_stacks: 1,
    stack_behavior: StackBehavior::Refresh,
};

// Buff'ı 5 saniye (tick) boyunca karakterin üzerine ekleyelim
status_manager.add_status(warcry, Some(5), 1);

// Zurnanın zırt dediği yer: Mevcut statların üzerine buff'ı UYGULA ve ÇATALLA!
let mut active_resolver = status_manager.get_active_resolver(&base_resolver);

// Bu okuma sonucunda artık +50 ATK bonusu dahil hesaplanmış değer döner!
let buffed_atk = active_resolver.resolve(&StatId::new("ATK"), &StatContext);

// Oyun Döngüsünde Süre Yönetimi:
status_manager.tick(); // Her çalıştığında süreleri azaltır, süresi biteni otomatik siler.
```

---

## 6. Combat Engine

Gelişmiş bir Soyut Sözdizimi Ağacı (AST) mimarisi olan Savaş Motoru; tamamen JSON (veya AST kodları) üzerinden tanımlanmış karmaşık hasar formüllerini işletir.

```rust
use zzstat::combat::{CombatEngine, Node, CombatContext};

// Hasar Formülü: "Saldıranın ATK'sından Savunanın DEF'ini Çıkar"
let damage_formula = Node::Subtract(
    Box::new(Node::Stat { target: "attacker".to_string(), stat: "ATK".to_string() }),
    Box::new(Node::Stat { target: "defender".to_string(), stat: "DEF".to_string() }),
);

// Formülü yönetecek savaş motoru ve taraflar (Resolver olarak verilir)
let mut combat = CombatEngine::new(damage_formula);
let mut combat_ctx = CombatContext::new(&attacker_resolver);
combat_ctx.add_target("defender", &defender_resolver);

// Rastgele sayı (RNG) üreteci. Bu örnekte %50 şans (Dodge/Crit) simüle etmek için sabit 0.5 dönüyoruz.
// Sabit döndüğü için tüm unit testleri 100% Deterministik olur!
let mut rand_generator = || 0.5; 

// Formülü hesapla
let damage = combat.evaluate(&combat_ctx, &mut rand_generator).unwrap();

println!("Hedefe {} hasar vuruldu!", damage);
```
