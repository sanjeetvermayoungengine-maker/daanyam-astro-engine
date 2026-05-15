# Yoga Detection — Phase 4 Roadmap

Engine v0.18+ ships classical yoga detection. The Patrika and Explainer both light up the yogas band only when this work lands.

Current state (May 2026):
- Scaffold landed: `crates/astro-vedic/src/yogas/` with the `Yoga` trait, `YogaChartFacts` data shape, registry, and 2 example detectors (Gajakesari, Chandra-Mangal).
- Inventory below is priority-ordered. Each yoga adds one file under `crates/astro-vedic/src/yogas/`, one entry in `registry::all_yoga_keys()` + `registry::detect_yogas()`, and one golden test fixture.
- Target: 30 yogas across 3 weeks (~10 per week, 2–4 hours each including the golden test).

## Definition of done for a single yoga

1. New file `yogas/<snake_name>.rs` with a `pub struct <PascalName>;` implementing the `Yoga` trait.
2. Detector unit tests (positive, negative, missing-data) in the same file.
3. Registered in `registry.rs` (both `all_yoga_keys` and `detect_yogas`).
4. Golden test in `crates/astro-vedic/tests/golden/yoga_<chart>.json` with a hand-verified birth chart from one of the well-known charts (Buddha, Vivekananda, Gandhi, Adi Shankaracharya — public-domain references).
5. Daanyam-voice line passes the editor's read: no prediction, no fear, Sanskrit nouns + everyday English verbs.

## Tier 1 — Highly cited, must-ship (10 yogas)

These are the names Indian seekers actually search for. Detection failure here is the most visible quality gap.

1. **Gajakesari** — Moon + Jupiter mutual kendra. *(Scaffolded.)*
2. **Chandra-Mangal** — Moon + Mars same rashi. *(Scaffolded.)*
3. **Budhaditya** — Sun + Mercury same rashi, Mercury not combust. Sun must be applying.
4. **Neecha Bhanga Raja Yoga** — A debilitated planet's dispositor in kendra/trikona from lagna or moon, OR debilitated planet exalted in navamsha.
5. **Vipreet Raja Yoga** — Lords of dusthana houses (6, 8, 12) in mutual exchange or aspecting each other.
6. **Dhana Yoga** — Lords of dhana houses (2, 5, 9, 11) in kendra/trikona to lagna with mutual aspect or exchange.
7. **Raja Yoga (kendra-trikona)** — Lords of trikona (1, 5, 9) and kendra (1, 4, 7, 10) in mutual exchange or conjunction.
8. **Kemadruma Yoga** — Moon with no graha in adjacent rashis (12th, 2nd from moon) and no graha in 2nd from moon, and Moon not in kendra from lagna. Treat as soft caution, not curse.
9. **Sunapha Yoga** — Graha (other than Sun) in 2nd from Moon.
10. **Anapha Yoga** — Graha (other than Sun) in 12th from Moon.

## Tier 2 — Pancha Mahapurusha (5 yogas)

The five "great-person" yogas. These are well-known to seekers and have crisp, easily testable rules.

11. **Ruchaka** — Mars in own (Mesha, Vrischika) or exalted (Makara) sign, in a kendra from lagna.
12. **Bhadra** — Mercury in own (Mithuna, Kanya) or exalted sign, in a kendra.
13. **Hamsa** — Jupiter in own (Dhanu, Meena) or exalted (Karka) sign, in a kendra.
14. **Malavya** — Venus in own (Vrischika, Tula) or exalted (Meena) sign, in a kendra.
15. **Shasha** — Saturn in own (Makara, Kumbha) or exalted (Tula) sign, in a kendra.

## Tier 3 — Common second-order yogas (10 yogas)

Less canonical but high-impact when present.

16. **Vipareeta Raja Yoga · Vimala** — Lord of 12 in 6/8/12.
17. **Vipareeta Raja Yoga · Sarala** — Lord of 8 in 6/8/12.
18. **Vipareeta Raja Yoga · Harsha** — Lord of 6 in 6/8/12.
19. **Akhanda Samrajya Yoga** — Lord of 2/9/11 in kendra from lagna with Jupiter in lagna, 2nd, 5th, 9th, or 11th.
20. **Lakshmi Yoga** — Lord of 9 in own house or exalted, with strong lagna lord, Jupiter or Venus in kendra.
21. **Saraswati Yoga** — Mercury, Jupiter, Venus together or in mutual kendra, with Jupiter in own/exalted in kendra/trikona.
22. **Adhi Yoga** — Benefics (Mercury, Jupiter, Venus) in 6, 7, 8 from Moon.
23. **Maha Bhagya Yoga** — Day birth: Sun, Moon, lagna in odd signs. Night birth: in even signs.
24. **Parijatha Yoga** — Lagna lord's dispositor's dispositor's dispositor in kendra (4-step exalted chain).
25. **Veshi/Vasi/Ubhayachari** — graha (not Moon) in 2nd / 12th / both sides of Sun (treat as 3 mini-detectors).

## Tier 4 — Less-cited but classically important (5 yogas)

These show up in serious readings; lower SEO demand, higher prestige.

26. **Sakata Yoga** — Moon in 6, 8, 12 from Jupiter. Soft framing.
27. **Daridra Yoga** — Lord of 11 in 6/8/12. Soft framing.
28. **Bhaskara Yoga** — Mercury in 2nd from Sun, Moon in 11th from Mercury, Jupiter in 5th or 9th from Moon.
29. **Kahala Yoga** — Lords of 4 and 9 in mutual kendra or exchange, with strong lagna lord.
30. **Pushkala Yoga** — Lord of Moon-sign with Moon's dispositor in kendra, strongly aspected by lagna lord.

## Charts to use for golden tests

Use public-domain birth charts to seed the golden tests. The list below is curated for breadth (different lagnas, different formations).

| Chart | Birth | Use for |
|---|---|---|
| Mahatma Gandhi | 1869-10-02, 07:08, Porbandar | Gajakesari (Moon-Jupiter mutual kendra), Saturn debilitation cases |
| Sri Ramakrishna | 1836-02-18, ~05:30, Kamarpukur | Raja Yoga, Dhana Yoga, Hamsa |
| Adi Shankaracharya | (traditional 788 CE) | Jupiter dominance, Sannyasa yogas |
| APJ Abdul Kalam | 1931-10-15, 01:00, Rameshwaram | Budhaditya, Akhanda Samrajya |
| Sachin Tendulkar | 1973-04-24, 17:15, Mumbai | Ruchaka, neecha bhanga of Saturn |

For each detector, pick the chart that has the yoga textbook-clean AND one chart that does NOT (negative test). Add both as JSON fixtures.

## Why this ordering

- Tier 1 covers the names that show up in seeker search queries. Detection failure here is the biggest quality gap.
- Tier 2 (Pancha Mahapurusha) ships fast — simple rules, easily testable.
- Tier 3 fills out the Patrika's "yogas band" without spending the budget on edge cases.
- Tier 4 ships if time remains; deferring these to a Phase 4.1 release is fine.

## Don't ship in v0.18

- Astakavarga-derived yogas. They need a separate Phase that adds Bhinnashtakavarga first.
- Combinations requiring D-30 (Trimshamsha) — wait for Phase 2 vargas to land.
- Combinations requiring Mantreshwara's Phaladeepika definitions where Parashara is ambiguous. Pick Parashara unless there's a single accepted reading.
