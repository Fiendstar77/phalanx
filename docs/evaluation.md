# Evaluation protocol — draft

## Design need

Players should be able to understand how personality and terrain change tactical behavior. A generative controller must improve a defined aspect of that experience while remaining legal, responsive, and inspectable.

## First experiments

1. Teamwork: compare low and high teamwork while holding other traits and combat statistics fixed across choke, ridge, and open-field arenas.
2. Caution: compare survival, damage, retreat frequency, and cover use under matched conditions.
3. Terrain interaction: compare aggressive and disciplined profiles across terrain presets.
4. Controller comparison: evaluate heuristics, utility AI, and utility AI plus selective LLM proposals after the deterministic baseline works.

## Experimental controls

- Save simulation version, configuration, arena, controller, seed, and model configuration.
- Use matched seeds and both spawn orientations; document how controllers consume randomness because a shared seed alone does not guarantee identical stochastic events.
- Separate development scenarios from held-out evaluation scenarios.
- Choose battle count and primary metric before evaluating; use pilot variance to inform sample size.
- Include timeouts, draws, rejected outputs, and model failures. Never silently discard failed trials.

## Telemetry contract — planned

Battle records: battle ID, seed, versions, map, traits, controller, winner or draw, duration, survivors, and timeout status.

Decision records: tick, agent ID, observed tactical state, valid actions, utility scores, selected action, controller source, validation outcome, and latency. LLM records additionally include model/settings, token use, and fallback reason.

Behavior measures: movement, damage, cover/high-ground use, retreat/regroup events, ally distance, and cohesion. Store explicit definitions and aggregation windows alongside results.

## Analysis

The battle is the unit of analysis. Report paired differences and confidence intervals across seed/side blocks. Select statistical tests based on outcome type and sampling assumptions; account for multiple comparisons if testing several hypotheses. Do not infer significance from a small illustrative demo.

Keep decision traces available for failure analysis. Stratify summaries by arena and controller; inspect whether an aggregate improvement hides failures on specific terrain.

## Integration criteria

Before LLM integration, specify a latency budget, legal-action requirement, fallback behavior, and maximum call budget. Validate proposals against the current world state. A timeout, malformed response, or illegal proposal falls back to utility AI.

## Player evaluation

Use a separate consent-based playtest protocol to assess decision readability, perceived agency, and usefulness of the inspector. Do not equate win rate with fun or player understanding.

## Current evidence

No experiments, playtests, training runs, or benchmarks have been performed. This document is a protocol, not a results report.
